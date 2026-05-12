#!/bin/bash
set -e

# ==============================================================================
# V6 Auto-Research Orchestrator — Cloud Run Deployment Script
# Deploys the production orchestrator as a serverless Cloud Run service
# ==============================================================================

PROJECT_ID="mopga-487511"
REGION="europe-west1"
SERVICE_NAME="rusty-sundials-autoresearch"
IMAGE="gcr.io/$PROJECT_ID/$SERVICE_NAME"
GCLOUD="/Users/xcallens/google-cloud-sdk/bin/gcloud"

echo "🚀 Deploying V6 Auto-Research Orchestrator to Cloud Run..."
echo "   Project: $PROJECT_ID"
echo "   Region:  $REGION"
echo "   Service: $SERVICE_NAME"

# 1. Ensure required APIs are enabled
echo "📦 Enabling required GCP APIs..."
$GCLOUD services enable run.googleapis.com \
    cloudbuild.googleapis.com \
    aiplatform.googleapis.com \
    artifactregistry.googleapis.com \
    --project=$PROJECT_ID

# 2. Build and push container
echo "🏗️ Building container via Cloud Build..."
$GCLOUD builds submit \
    --tag $IMAGE \
    --project=$PROJECT_ID \
    -f deploy/cloudrun/Dockerfile \
    .

# 3. Deploy to Cloud Run
echo "☁️ Deploying to Cloud Run..."
$GCLOUD run deploy $SERVICE_NAME \
    --image $IMAGE \
    --region $REGION \
    --project $PROJECT_ID \
    --platform managed \
    --allow-unauthenticated \
    --memory 2Gi \
    --cpu 1 \
    --timeout 300 \
    --max-instances 1 \
    --set-env-vars="PROJECT_ID=$PROJECT_ID,VERTEX_AI_REGION=$REGION,GEMINI_API_KEY=$GEMINI_API_KEY"

echo ""
echo "✅ Deployment complete!"
echo "   To trigger a run:"
echo "   curl -X POST https://$SERVICE_NAME-xxxxx.run.app/run"
echo ""
echo "   To check health:"
echo "   curl https://$SERVICE_NAME-xxxxx.run.app/health"
