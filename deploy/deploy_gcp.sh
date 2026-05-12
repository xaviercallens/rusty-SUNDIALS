#!/bin/bash
set -e

# ==============================================================================
# GCP Serverless A100 & Cloud Run Deployment Script
# Target: Sub-$100/mo Autonomous Discovery Engine
# ==============================================================================

PROJECT_ID="your-gcp-project-id"
REGION="us-central1"
ORCHESTRATOR_IMAGE="gcr.io/$PROJECT_ID/rusty-sundials-orchestrator"

echo "🚀 Bootstrapping Serverless Exascale Infrastructure..."

# 1. Ensure gcloud is installed
if ! command -v gcloud &> /dev/null; then
    echo "❌ gcloud CLI not found. Please install: https://cloud.google.com/sdk/docs/install"
    exit 1
fi

gcloud config set project $PROJECT_ID
gcloud services enable run.googleapis.com aiplatform.googleapis.com artifactregistry.googleapis.com

# 2. Build and Push Orchestrator to Cloud Run
echo "📦 Building lightweight Orchestrator container..."
gcloud builds submit --tag $ORCHESTRATOR_IMAGE -f deploy/Dockerfile.orchestrator .

# 3. Deploy Orchestrator as a Serverless Cloud Run Job (Zero cost when idle)
echo "☁️ Deploying Orchestrator to Cloud Run Jobs..."
gcloud run jobs create rusty-sundials-agent \
    --image $ORCHESTRATOR_IMAGE \
    --region $REGION \
    --tasks 1 \
    --max-retries 0 \
    --memory 2Gi \
    --cpu 1 \
    --set-env-vars=VERTEX_AI_REGION=$REGION,PROJECT_ID=$PROJECT_ID

echo "✅ Orchestrator deployed! You can trigger a run with: gcloud run jobs execute rusty-sundials-agent"

# 4. Instructions for Vertex AI Open-Weights endpoints
echo ""
echo "⚠️  ACTION REQUIRED for Serverless A100 GPUs:"
echo "Please follow docs/GCP_SERVERLESS_DEPLOYMENT.md to deploy Qwen3.6-Math and CodeBERT via Vertex AI endpoints scaling to 0."
