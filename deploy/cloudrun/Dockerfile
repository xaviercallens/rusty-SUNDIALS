# ============================================================================
# V6 Auto-Research Orchestrator — Production Cloud Run Dockerfile
# Lightweight CPU-only container that routes LLM calls to Vertex AI Gemini
# ============================================================================
FROM python:3.12-slim

# System deps (minimal)
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install Python deps first (for Docker layer caching)
COPY deploy/cloudrun/requirements.txt /app/requirements.txt
RUN pip install --no-cache-dir -r requirements.txt

# Copy the orchestrator agent
COPY autoresearch_agent/ /app/autoresearch_agent/

# Copy the Cloud Run entrypoint
COPY deploy/cloudrun/main.py /app/main.py

WORKDIR /app

# Cloud Run expects PORT env var
ENV PORT=8080

# Run the HTTP server entrypoint
CMD ["python3", "main.py"]
