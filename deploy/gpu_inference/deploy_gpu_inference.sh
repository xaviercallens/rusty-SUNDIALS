#!/usr/bin/env bash
# =============================================================================
# rusty-SUNDIALS v10 — GPU Inference Deployment Script
# =============================================================================
# Deploys the Qwen3-8B + Qwen2.5-Coder-7B vLLM server to GCP
#
# TWO deployment targets (choose one via --target flag):
#
#   A) Vertex AI Custom Endpoint  (A100 80GB, scale-to-zero, best quality)
#      Cost: ~$3.93/hr A100 (billed per second, scales to $0 when idle)
#      Cold start: ~5-10 min (models baked into image)
#
#   B) Cloud Run GPU (L4 24GB, FP8 quant, faster cold start)
#      Cost: ~$0.55/hr L4 (billed per ms, true scale-to-zero)
#      Cold start: ~3-5 min (models baked into image)
#
# Usage:
#   ./deploy_gpu_inference.sh --target vertex   # Vertex AI A100
#   ./deploy_gpu_inference.sh --target cloudrun # Cloud Run L4
#   ./deploy_gpu_inference.sh --target both     # Deploy both
# =============================================================================
set -euo pipefail

# ── Configuration ─────────────────────────────────────────────────────────────
PROJECT_ID="${PROJECT_ID:-mopga-487511}"
REGION="${REGION:-europe-west1}"
AR_REPO="rusty-sundials-gpu"
IMAGE_NAME="vllm-inference-server"
IMAGE_TAG="${IMAGE_TAG:-v10.0.0}"
IMAGE_FULL="$REGION-docker.pkg.dev/$PROJECT_ID/$AR_REPO/$IMAGE_NAME:$IMAGE_TAG"
GCLOUD="${GCLOUD:-gcloud}"

# Model config
HF_TOKEN="${HF_TOKEN:-}"
GPU_VERTEX_MACHINE="a2-highgpu-1g"   # 1× A100 80GB, 12 vCPU, 85GB RAM
GPU_VERTEX_ACCEL="NVIDIA_TESLA_A100"
GPU_CR_TYPE="nvidia-l4"               # Cloud Run GPU (L4 24GB)

# Service names
VERTEX_ENDPOINT_NAME="rusty-sundials-v10-inference"
CLOUDRUN_SERVICE_NAME="rusty-sundials-v10-gpu"

# ── Parse arguments ────────────────────────────────────────────────────────────
TARGET="${1:-vertex}"
if [ "${1:-}" = "--target" ]; then
    TARGET="${2:-vertex}"
fi

echo "================================================================"
echo " rusty-SUNDIALS v10 GPU Inference Deployment"
echo " Project:   $PROJECT_ID"
echo " Region:    $REGION"
echo " Image:     $IMAGE_FULL"
echo " Target:    $TARGET"
echo "================================================================"

# ── Step 0: Enable required APIs ──────────────────────────────────────────────
echo ""
echo "📦 [0/5] Enabling GCP APIs..."
$GCLOUD services enable \
    run.googleapis.com \
    aiplatform.googleapis.com \
    cloudbuild.googleapis.com \
    artifactregistry.googleapis.com \
    compute.googleapis.com \
    --project="$PROJECT_ID" --quiet

# ── Step 1: Create Artifact Registry repo ─────────────────────────────────────
echo ""
echo "🗄️  [1/5] Creating Artifact Registry repository..."
$GCLOUD artifacts repositories create "$AR_REPO" \
    --repository-format=docker \
    --location="$REGION" \
    --description="rusty-SUNDIALS v10 GPU inference container" \
    --project="$PROJECT_ID" 2>/dev/null \
    || echo "   (Repository already exists — skipping)"

# Configure Docker auth
$GCLOUD auth configure-docker "$REGION-docker.pkg.dev" --quiet

# ── Step 2: Build container via Cloud Build ───────────────────────────────────
echo ""
echo "🏗️  [2/5] Building container via Cloud Build..."
echo "   Image: $IMAGE_FULL"
echo "   NOTE: Build includes model downloads (~30 GB) — takes 20-40 min first time"

# Pass HF_TOKEN as substitution (never in Dockerfile ARG without substitution)
$GCLOUD builds submit \
    --tag="$IMAGE_FULL" \
    --project="$PROJECT_ID" \
    --machine-type="e2-highcpu-32" \
    --disk-size=200 \
    --timeout="7200s" \
    --substitutions="_HF_TOKEN=$HF_TOKEN" \
    --build-arg "HF_TOKEN=$HF_TOKEN" \
    -f deploy/gpu_inference/Dockerfile.vllm \
    .

echo "✅ Container built: $IMAGE_FULL"

# ── Step 3a: Deploy to Vertex AI (A100) ───────────────────────────────────────
if [[ "$TARGET" == "vertex" || "$TARGET" == "both" ]]; then
    echo ""
    echo "🧠 [3/5] Deploying to Vertex AI (A100 80GB, scale-to-zero)..."

    # Create model resource
    VERTEX_MODEL_NAME="qwen3-qwen-coder-vllm-v10"
    $GCLOUD ai models upload \
        --region="$REGION" \
        --display-name="$VERTEX_MODEL_NAME" \
        --container-image-uri="$IMAGE_FULL" \
        --container-ports=8080 \
        --container-health-route="/health" \
        --container-predict-route="/v1/chat/completions" \
        --container-env-vars="GPU_TYPE=A100,VLLM_QUANT=,PORT=8080,HF_TOKEN=$HF_TOKEN" \
        --project="$PROJECT_ID" \
        2>/dev/null || echo "   (Model already exists — updating)"

    # Create or update endpoint
    echo "   Creating Vertex AI endpoint..."
    ENDPOINT_ID=$($GCLOUD ai endpoints list \
        --region="$REGION" \
        --project="$PROJECT_ID" \
        --filter="displayName=$VERTEX_ENDPOINT_NAME" \
        --format="value(name)" | head -1)

    if [ -z "$ENDPOINT_ID" ]; then
        ENDPOINT_ID=$($GCLOUD ai endpoints create \
            --region="$REGION" \
            --display-name="$VERTEX_ENDPOINT_NAME" \
            --project="$PROJECT_ID" \
            --format="value(name)")
        echo "   Created endpoint: $ENDPOINT_ID"
    fi

    # Deploy model to endpoint with scale-to-zero
    MODEL_ID=$($GCLOUD ai models list \
        --region="$REGION" \
        --project="$PROJECT_ID" \
        --filter="displayName=$VERTEX_MODEL_NAME" \
        --format="value(name)" | head -1)

    $GCLOUD ai endpoints deploy-model "$ENDPOINT_ID" \
        --region="$REGION" \
        --project="$PROJECT_ID" \
        --model="$MODEL_ID" \
        --machine-type="$GPU_VERTEX_MACHINE" \
        --accelerator="type=$GPU_VERTEX_ACCEL,count=1" \
        --min-replica-count=0 \
        --max-replica-count=2 \
        --display-name="v10-qwen3-coder" \
        --traffic-split=0=100

    VERTEX_URL="https://$REGION-aiplatform.googleapis.com/v1/$ENDPOINT_ID:predict"
    echo ""
    echo "✅ Vertex AI endpoint deployed!"
    echo "   Endpoint: $VERTEX_URL"
    echo "   Scale-to-zero: YES (min=0 replicas)"
    echo "   GPU: A100 80GB @ ~\$3.93/hr (billed per second)"
    echo ""
    echo "   Set env var for orchestrator:"
    echo "   export VLLM_INFERENCE_URL=$VERTEX_URL"
fi

# ── Step 3b: Deploy to Cloud Run GPU (L4) ─────────────────────────────────────
if [[ "$TARGET" == "cloudrun" || "$TARGET" == "both" ]]; then
    echo ""
    echo "☁️  [3/5] Deploying to Cloud Run GPU (L4 24GB, FP8, true scale-to-zero)..."

    $GCLOUD run deploy "$CLOUDRUN_SERVICE_NAME" \
        --image="$IMAGE_FULL" \
        --region="$REGION" \
        --project="$PROJECT_ID" \
        --platform=managed \
        --no-allow-unauthenticated \
        --gpu=1 \
        --gpu-type="$GPU_CR_TYPE" \
        --memory=32Gi \
        --cpu=8 \
        --timeout=3600 \
        --concurrency=4 \
        --min-instances=0 \
        --max-instances=2 \
        --set-env-vars="GPU_TYPE=L4,VLLM_QUANT=fp8,PORT=8080,HF_TOKEN=$HF_TOKEN,HF_HUB_CACHE=/model-cache" \
        --set-env-vars="MAX_MODEL_LEN=16384"

    CR_URL=$($GCLOUD run services describe "$CLOUDRUN_SERVICE_NAME" \
        --region="$REGION" --project="$PROJECT_ID" --format="value(status.url)")

    echo ""
    echo "✅ Cloud Run GPU service deployed!"
    echo "   URL: $CR_URL"
    echo "   GPU: L4 24GB (FP8 quantization)"
    echo "   Scale-to-zero: YES"
    echo "   Cost: ~\$0.55/hr when active (billed per ms)"
    echo ""
    echo "   Set env var for orchestrator:"
    echo "   export VLLM_INFERENCE_URL=$CR_URL"
    echo ""
    echo "   ⚠️  Authentication required (no-allow-unauthenticated):"
    echo "   Use: gcloud auth print-identity-token for Bearer token"
fi

# ── Step 4: Update orchestrator env var ───────────────────────────────────────
echo ""
echo "🔧 [4/5] Update autoresearch_agent/.env with VLLM_INFERENCE_URL"
cat >> /tmp/v10_env_additions.txt << 'ENVEOF'

# ── rusty-SUNDIALS v10 GPU Inference ────────────────────────────────────────
# Uncomment the deployment you are using:
# VLLM_INFERENCE_URL=https://europe-west1-aiplatform.googleapis.com/v1/<ENDPOINT>:predict
# VLLM_INFERENCE_URL=https://rusty-sundials-v10-gpu-xxxxx-ew.a.run.app

# GPU configuration
GPU_TYPE=A100  # or L4
VLLM_QUANT=    # empty=fp16 (A100), fp8 (L4)

# HuggingFace token for private models
HF_TOKEN=hf_...

# Gwen3-8B thinking mode budget
QWEN3_THINKING_BUDGET=8192
ENVEOF
echo "   Appended to /tmp/v10_env_additions.txt — copy to your .env file"

# ── Step 5: Summary ────────────────────────────────────────────────────────────
echo ""
echo "================================================================"
echo " ✅ DEPLOYMENT COMPLETE"
echo "================================================================"
echo ""
echo " Model stack:"
echo "   🔵 Qwen/Qwen3-8B            — math/proof/peer-review (thinking)"
echo "   🟢 Qwen/Qwen2.5-Coder-7B-Instruct — Rust code synthesis"
echo "   🟡 microsoft/codebert-base  — code semantic embeddings"
echo "   🔴 DeepProbLog + ProbLog    — neuro-symbolic physics gatekeeper"
echo ""
echo " Cost estimate per discovery cycle:"
echo "   A100: ~3-5 min @ \$3.93/hr = ~\$0.20-0.35"
echo "   L4:   ~5-8 min @ \$0.55/hr = ~\$0.05-0.07"
echo ""
echo " To test the endpoint:"
echo '   curl -X POST $VLLM_INFERENCE_URL/v1/chat/completions \'
echo '     -H "Content-Type: application/json" \'
echo '     -d '"'"'{"messages":[{"role":"user","content":"Prove: 6 ≤ 7 in Lean 4"}],"task_type":"proof"}'"'"
echo ""
