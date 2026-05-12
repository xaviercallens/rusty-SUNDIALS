# Security Posture & Credential Management

To prevent unauthorized usage, quotas, and surprise billing costs on the GCP infrastructure, it is critical to move away from raw API keys (like `GEMINI_API_KEY`) and use modern **Application Default Credentials (ADC)** via Google Cloud IAM.

## 1. Restricting the Legacy Gemini API Key
If you generated a legacy API key in Google AI Studio, you must restrict it so it cannot be abused if leaked:
1. Go to the **Google Cloud Console** > **APIs & Services** > **Credentials**.
2. Click on your `Generative Language API Key`.
3. Under **Key Restrictions** > **API Restrictions**, select **Restrict key**.
4. Check ONLY **Generative Language API**.
5. Save.

## 2. Switching to GCP IAM (The Secure Route)
The `rusty-SUNDIALS` v6 orchestrator now natively supports GCP's Vertex AI SDK. This completely removes the need for API keys. Instead, it relies on Google Cloud IAM (Identity and Access Management) Service Accounts.

### Running Locally (Mac M2)
Instead of exporting an API key, authenticate your shell directly with GCP:
```bash
gcloud auth application-default login
export PROJECT_ID="your-gcp-project-id"
export VERTEX_AI_REGION="us-central1"
```
The orchestrator (`hypothesizer_llm.py`) will automatically detect these variables and switch to the `vertexai` library, completely bypassing the `google.generativeai` API key requirement.

### Running in Cloud Run (Serverless)
When the orchestrator runs in Cloud Run (as set up in `deploy_gcp.sh`), it inherently possesses the identity of its attached Service Account. 
1. Ensure the Cloud Run Service Account has the **Vertex AI User** role.
2. The environment variables `PROJECT_ID` and `VERTEX_AI_REGION` are automatically injected during deployment.
3. The orchestrator will securely invoke Gemini 1.5/2.0 without any API keys embedded in the container!

By removing hardcoded API keys and utilizing IAM, you eliminate the risk of billing exploits.
