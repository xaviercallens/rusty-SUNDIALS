# GCP Serverless Auto-Research Architecture (< $100 / month)

To run the `rusty-SUNDIALS` autonomous research agent 24/7 without incurring the massive baseline costs of renting dedicated A100 instances ($1000+/mo), we utilize a strictly **Serverless (Scale-to-Zero)** architecture on Google Cloud.

## The Architecture
1. **The Orchestrator (CPU)**: Deployed to **Google Cloud Run Jobs**. It only runs when triggered (e.g., via Cloud Scheduler once a day). Costs $\sim \$0.000024$ per second.
2. **The Intuition Engine (Gemini)**: Uses the native GCP Vertex AI Gemini 1.5/2.0 API. Extremely cheap per token.
3. **The Proof & Syntax Engines (A100 GPU)**: Qwen-Math-72B and CodeBERT are deployed to **Vertex AI Endpoints**. We configure these endpoints to scale down to **0 nodes** when idle. 

By scaling to 0, you only pay for the exact seconds the A100 GPU is awake processing Lean 4 proofs, keeping your total bill well under $100/mo.

---

## Step 1: Deploying the Orchestrator
We have containerized the Python agent and DeepProbLog/Lean 4 toolchain.

1. Install the `gcloud` CLI on your Mac M2:
   ```bash
   brew install --cask google-cloud-sdk
   gcloud auth login
   ```
2. Edit `deploy/deploy_gcp.sh` and set `PROJECT_ID="your-gcp-project-id"`.
3. Run the deployment script:
   ```bash
   chmod +x deploy/deploy_gcp.sh
   ./deploy/deploy_gcp.sh
   ```

---

## Step 2: Deploying Serverless Models to Vertex AI (A100)
To avoid running local LLMs on your Mac M2, we push them to Vertex AI using vLLM.

### A. Deploy Qwen3.6-Math (The Lean 4 Prover)
Execute this `gcloud` command to create a serverless vLLM endpoint:

```bash
gcloud ai endpoints create \
  --region=us-central1 \
  --display-name=qwen-math-serverless

# Deploy the HuggingFace model using the pre-built vLLM image
gcloud ai endpoints deploy-model qwen-math-serverless \
  --region=us-central1 \
  --model=hf://Qwen/Qwen3.6-Math-72B \
  --display-name=qwen-math-vllm \
  --machine-type=a2-highgpu-1g \
  --accelerator-type=NVIDIA_TESLA_A100 \
  --accelerator-count=1 \
  --min-replica-count=0 \
  --max-replica-count=1 \
  --container-image-uri=us-docker.pkg.dev/vertex-ai/vertex-vision-model-garden-dockers/pytorch-vllm-serve:20240220
```

### B. Deploy CodeBERT (The Rust Synthesizer)
```bash
gcloud ai endpoints create \
  --region=us-central1 \
  --display-name=codebert-serverless

gcloud ai endpoints deploy-model codebert-serverless \
  --region=us-central1 \
  --model=hf://microsoft/codebert-base \
  --display-name=codebert-vllm \
  --machine-type=n1-standard-4 \
  --accelerator-type=NVIDIA_TESLA_T4 \
  --accelerator-count=1 \
  --min-replica-count=0 \
  --max-replica-count=1 \
  --container-image-uri=us-docker.pkg.dev/vertex-ai/vertex-vision-model-garden-dockers/pytorch-hf-serve:latest
```

## Step 3: Configure Orchestrator to ping Vertex AI
Once the endpoints are live, update your `autoresearch_agent` python scripts to query the Vertex AI endpoint URL instead of the local Mac M2 `localhost:8000`.

Because `min-replica-count=0`, the A100s will sleep until the orchestrator sends a proof request. The first request will have a "Cold Start" penalty of ~2-3 minutes while the A100 spins up, but subsequent proof iterations will be lightning fast, keeping your total monthly bill negligible.
