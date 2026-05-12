"""
Cloud Run HTTP Entrypoint for V6 Auto-Research Orchestrator.
Provides a /run endpoint that triggers a single orchestrator loop,
and a /health endpoint for readiness probes.
"""
import os
import sys
import json
import time
import traceback
from http.server import HTTPServer, BaseHTTPRequestHandler
from datetime import datetime

# Add the autoresearch_agent to the path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "autoresearch_agent"))

from autoresearch_agent.orchestrator_prod import ProdOrchestrator


class OrchestratorHandler(BaseHTTPRequestHandler):
    """HTTP handler that wraps the Auto-Research orchestrator."""

    orchestrator = None

    def do_GET(self):
        if self.path == "/health":
            self._respond(200, {"status": "healthy", "version": "v6.0.0"})
        elif self.path == "/":
            self._respond(200, {
                "service": "rusty-sundials-autoresearch-v6",
                "description": "Formally Verified Neuro-Symbolic Auto-Discovery Engine",
                "endpoints": {
                    "/health": "Health check",
                    "/run": "POST — Execute one auto-research loop"
                }
            })
        else:
            self._respond(404, {"error": "Not found"})

    def do_POST(self):
        if self.path == "/run":
            self._run_orchestrator()
        else:
            self._respond(404, {"error": "Not found"})

    def _run_orchestrator(self):
        """Execute the auto-research orchestrator and return results."""
        start = time.time()
        try:
            # Parse optional config from request body
            content_length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(content_length) if content_length > 0 else b"{}"
            config = json.loads(body) if body else {}

            max_loops = config.get("max_loops", 5)

            orch = ProdOrchestrator(max_loops=max_loops)
            result = orch.run_loop()

            elapsed = time.time() - start
            result["elapsed_seconds"] = round(elapsed, 2)
            result["timestamp"] = datetime.utcnow().isoformat() + "Z"

            self._respond(200, result)

        except Exception as e:
            elapsed = time.time() - start
            self._respond(500, {
                "error": str(e),
                "traceback": traceback.format_exc(),
                "elapsed_seconds": round(elapsed, 2)
            })

    def _respond(self, status_code, data):
        self.send_response(status_code)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps(data, indent=2).encode())


def main():
    port = int(os.environ.get("PORT", 8080))
    server = HTTPServer(("0.0.0.0", port), OrchestratorHandler)
    print(f"🚀 V6 Auto-Research Orchestrator listening on port {port}")
    print(f"   POST /run to trigger a discovery loop")
    print(f"   GET /health for health check")
    server.serve_forever()


if __name__ == "__main__":
    main()
