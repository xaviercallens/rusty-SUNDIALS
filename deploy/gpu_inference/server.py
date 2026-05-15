"""
rusty-SUNDIALS v10 — GPU Inference Gateway Server
==================================================
FastAPI server that:
  1. Manages two vLLM subprocess workers:
       - Qwen/Qwen3-8B      (port 8001) — math, reasoning, peer review, proof tactics
       - Qwen/Qwen2.5-Coder-7B-Instruct (port 8002) — Rust code synthesis
  2. Exposes a unified OpenAI-compatible API at :8080
  3. Routes requests to the appropriate model based on `task_type`:
       math/reasoning/review/proof → Qwen3-8B (thinking mode ON)
       code/rust/synthesize        → Qwen2.5-Coder-7B
  4. Provides /health, /metrics, /neuro-symbolic endpoints
  5. Runs DeepProbLog gatekeeper in-process

Cloud Run GPU L4 (24GB): use quantization=fp8 for both models (~15GB total)
Vertex AI A100 (80GB):   run both at fp16 for maximum quality (~38GB total)
"""

from __future__ import annotations
import asyncio
import logging
import os
import subprocess
import sys
import time
from contextlib import asynccontextmanager
from typing import Optional

import httpx
import uvicorn
from fastapi import FastAPI, HTTPException, BackgroundTasks
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(message)s")
logger = logging.getLogger(__name__)

# ── Configuration ─────────────────────────────────────────────────────────────

GPU_TYPE = os.environ.get("GPU_TYPE", "L4")           # "L4" | "A100"
QUANTIZATION = os.environ.get("VLLM_QUANT", "fp8" if GPU_TYPE == "L4" else None)
MODEL_CACHE = os.environ.get("HF_HUB_CACHE", "/model-cache")
HF_TOKEN = os.environ.get("HF_TOKEN", "")
MAX_MODEL_LEN = int(os.environ.get("MAX_MODEL_LEN", "32768"))

# Model identifiers
QWEN3_MODEL = "Qwen/Qwen3-8B"
QWEN_CODER_MODEL = "Qwen/Qwen2.5-Coder-7B-Instruct"
CODEBERT_MODEL = "microsoft/codebert-base"

# vLLM subprocess ports (internal, not exposed)
QWEN3_PORT = 8001
QWEN_CODER_PORT = 8002

# ── vLLM subprocess management ────────────────────────────────────────────────

_vllm_procs: dict[str, subprocess.Popen] = {}


def _build_vllm_cmd(model: str, port: int, thinking: bool = False) -> list[str]:
    """Build vLLM serve command for a given model."""
    cmd = [
        sys.executable, "-m", "vllm.entrypoints.openai.api_server",
        "--model", model,
        "--port", str(port),
        "--host", "0.0.0.0",
        "--max-model-len", str(MAX_MODEL_LEN),
        "--download-dir", MODEL_CACHE,
        "--served-model-name", model.split("/")[-1],
        "--trust-remote-code",
        "--dtype", "auto",
        "--enforce-eager",           # avoids CUDA graph issues on first cold start
        "--max-num-seqs", "8",       # conservative: share GPU with sibling model
        "--disable-log-requests",
    ]
    if QUANTIZATION:
        cmd += ["--quantization", QUANTIZATION]
    if HF_TOKEN:
        cmd += ["--token", HF_TOKEN]
    if thinking:
        # Qwen3 thinking mode: enable reasoning parser
        cmd += [
            "--enable-reasoning",
            "--reasoning-parser", "qwen3",
        ]
    return cmd


async def start_vllm_workers():
    """Start both vLLM workers as subprocesses."""
    logger.info("🚀 Starting vLLM workers...")

    # Qwen3-8B: math + reasoning + peer review (thinking mode)
    cmd3 = _build_vllm_cmd(QWEN3_MODEL, QWEN3_PORT, thinking=True)
    logger.info(f"[Qwen3-8B] {' '.join(cmd3[:6])}…")
    _vllm_procs["qwen3"] = subprocess.Popen(
        cmd3, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        env={**os.environ, "CUDA_VISIBLE_DEVICES": "0"},
    )

    # Wait briefly then start Coder on same GPU (vLLM manages memory internally)
    await asyncio.sleep(5)

    # Qwen2.5-Coder-7B: Rust code synthesis
    cmd_coder = _build_vllm_cmd(QWEN_CODER_MODEL, QWEN_CODER_PORT, thinking=False)
    logger.info(f"[Qwen2.5-Coder] {' '.join(cmd_coder[:6])}…")
    _vllm_procs["coder"] = subprocess.Popen(
        cmd_coder, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
        env={**os.environ, "CUDA_VISIBLE_DEVICES": "0"},
    )

    # Wait for both workers to be ready
    logger.info("⏳ Waiting for vLLM workers to load models (may take 2-5 min on cold start)…")
    for name, port in [("qwen3", QWEN3_PORT), ("coder", QWEN_CODER_PORT)]:
        await _wait_for_vllm(name, port, timeout=360)

    logger.info("✅ Both vLLM workers ready.")


async def _wait_for_vllm(name: str, port: int, timeout: int = 300):
    """Poll vLLM health endpoint until ready or timeout."""
    deadline = time.time() + timeout
    async with httpx.AsyncClient() as client:
        while time.time() < deadline:
            try:
                resp = await client.get(f"http://localhost:{port}/health", timeout=5)
                if resp.status_code == 200:
                    logger.info(f"✅ [{name}] vLLM worker ready on port {port}")
                    return
            except Exception:
                pass
            await asyncio.sleep(5)
    logger.error(f"❌ [{name}] vLLM worker on port {port} did not become ready in {timeout}s")


def stop_vllm_workers():
    for name, proc in _vllm_procs.items():
        logger.info(f"Stopping [{name}] vLLM worker…")
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()


# ── CodeBERT semantic embeddings (loaded in-process) ─────────────────────────

_codebert = None


def load_codebert():
    """Load CodeBERT once at startup for semantic code similarity."""
    global _codebert
    try:
        from transformers import AutoTokenizer, AutoModel
        import torch
        logger.info(f"[CodeBERT] Loading {CODEBERT_MODEL}…")
        tokenizer = AutoTokenizer.from_pretrained(
            CODEBERT_MODEL, cache_dir=MODEL_CACHE)
        model = AutoModel.from_pretrained(
            CODEBERT_MODEL, cache_dir=MODEL_CACHE)
        model.eval()
        if torch.cuda.is_available():
            model = model.cuda()
        _codebert = {"tokenizer": tokenizer, "model": model}
        logger.info("✅ [CodeBERT] Loaded.")
    except Exception as exc:
        logger.warning(f"[CodeBERT] Load failed: {exc} — semantic embeddings unavailable.")
        _codebert = None


def get_code_embedding(code: str) -> list[float] | None:
    """Return a 768-dim CodeBERT embedding for a code snippet."""
    if _codebert is None:
        return None
    import torch
    tok = _codebert["tokenizer"]
    mdl = _codebert["model"]
    inputs = tok(code[:512], return_tensors="pt", truncation=True, padding=True)
    if torch.cuda.is_available():
        inputs = {k: v.cuda() for k, v in inputs.items()}
    with torch.no_grad():
        outputs = mdl(**inputs)
    embedding = outputs.last_hidden_state[:, 0, :].squeeze().cpu().tolist()
    return embedding


# ── DeepProbLog neuro-symbolic gatekeeper ────────────────────────────────────

def run_deepproblog_check(hypothesis: dict) -> tuple[bool, str]:
    """
    Run the DeepProbLog physics gatekeeper.
    Falls back to pure-Python logic if DeepProbLog is unavailable.
    """
    try:
        from problog.program import PrologString
        from problog import get_evaluatable

        div_b = hypothesis.get("preserves_magnetic_divergence", False)
        energy = hypothesis.get("conserves_energy", False)
        speedup = hypothesis.get("expected_speedup_factor", 0)

        # ProbLog program encoding the physics constraints
        program = PrologString(f"""
            % xMHD Physics invariants encoded as probabilistic logic
            valid_hypothesis :-
                preserves_div_b,
                conserves_energy,
                valid_speedup.

            preserves_div_b :- {str(div_b).lower()}.
            conserves_energy :- {str(energy).lower()}.
            valid_speedup :- {str(0 < speedup < 1e5).lower()}.

            query(valid_hypothesis).
        """)

        result = get_evaluatable().create_from(program).evaluate()
        for query, prob in result.items():
            if prob > 0.5:
                return True, f"DeepProbLog: valid_hypothesis p={prob:.3f}"
            else:
                return False, f"DeepProbLog: valid_hypothesis p={prob:.3f} — below threshold"
    except Exception as exc:
        logger.warning(f"[DeepProbLog] Unavailable ({exc}), using fallback logic.")
        # Deterministic fallback (same as physics_validator_v10.py Gate 3)
        if not hypothesis.get("preserves_magnetic_divergence", False):
            return False, "∇·B ≠ 0: Maxwell violation."
        if not hypothesis.get("conserves_energy", False):
            return False, "Energy not conserved: Hamiltonian violation."
        return True, "Physics constraints satisfied (fallback logic)."


# ── FastAPI app ───────────────────────────────────────────────────────────────

@asynccontextmanager
async def lifespan(app: FastAPI):
    # Startup
    load_codebert()
    await start_vllm_workers()
    yield
    # Shutdown
    stop_vllm_workers()


app = FastAPI(
    title="rusty-SUNDIALS v10 GPU Inference Gateway",
    description="Unified OpenAI-compatible API for Qwen3-8B + Qwen2.5-Coder + CodeBERT + DeepProbLog",
    version="10.0.0",
    lifespan=lifespan,
)
app.add_middleware(CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"])


# ── Request / Response schemas ────────────────────────────────────────────────

class ChatMessage(BaseModel):
    role: str
    content: str


class ChatRequest(BaseModel):
    messages: list[ChatMessage]
    task_type: str = Field(
        default="auto",
        description="'math'|'proof'|'review'|'code'|'rust'|'synthesize'|'auto'",
    )
    model: Optional[str] = None
    temperature: float = 0.6
    max_tokens: int = 4096
    thinking_budget: Optional[int] = None   # tokens for Qwen3 thinking mode
    enable_thinking: bool = True


class EmbedRequest(BaseModel):
    code: str


class NeuroSymbolicRequest(BaseModel):
    hypothesis: dict


# ── Route: unified chat completion ────────────────────────────────────────────

CODE_TASKS = {"code", "rust", "synthesize", "codebert"}
MATH_TASKS = {"math", "proof", "review", "reasoning", "physics", "lean4"}


@app.post("/v1/chat/completions")
async def chat_completions(req: ChatRequest):
    """
    Route to the appropriate vLLM backend based on task_type.
    Mirrors the OpenAI /v1/chat/completions schema.
    """
    task = req.task_type.lower()

    # Auto-detect from message content
    if task == "auto":
        last_msg = req.messages[-1].content.lower() if req.messages else ""
        task = "code" if any(kw in last_msg for kw in
                              ["rust", "fn ", "impl ", "struct ", "use sundials"]) else "math"

    if task in CODE_TASKS:
        target_port = QWEN_CODER_PORT
        model_name = QWEN_CODER_MODEL.split("/")[-1]
        use_thinking = False
    else:
        target_port = QWEN3_PORT
        model_name = QWEN3_MODEL.split("/")[-1]
        use_thinking = req.enable_thinking

    payload = {
        "model": model_name,
        "messages": [m.model_dump() for m in req.messages],
        "temperature": req.temperature,
        "max_tokens": req.max_tokens,
    }
    if use_thinking and req.thinking_budget:
        payload["chat_template_kwargs"] = {"thinking_budget": req.thinking_budget}
        payload["extra_body"] = {"thinking": {"type": "enabled",
                                               "budget_tokens": req.thinking_budget}}

    async with httpx.AsyncClient(timeout=300) as client:
        try:
            resp = await client.post(
                f"http://localhost:{target_port}/v1/chat/completions",
                json=payload,
            )
            resp.raise_for_status()
            result = resp.json()
            # Tag the response with which model was used
            result["_gateway"] = {
                "task_type": task,
                "backend_port": target_port,
                "model_used": model_name,
                "thinking_enabled": use_thinking,
            }
            return result
        except httpx.HTTPStatusError as exc:
            raise HTTPException(status_code=exc.response.status_code,
                                detail=exc.response.text)
        except Exception as exc:
            raise HTTPException(status_code=503, detail=f"vLLM backend error: {exc}")


# ── Route: code embeddings via CodeBERT ──────────────────────────────────────

@app.post("/v1/embeddings/code")
async def code_embedding(req: EmbedRequest):
    """Return a 768-dim CodeBERT semantic embedding for a code snippet."""
    embedding = get_code_embedding(req.code)
    if embedding is None:
        raise HTTPException(status_code=503, detail="CodeBERT not loaded.")
    return {
        "model": CODEBERT_MODEL,
        "embedding": embedding,
        "dimensions": len(embedding),
        "code_length": len(req.code),
    }


# ── Route: neuro-symbolic physics check ───────────────────────────────────────

@app.post("/v1/neuro-symbolic/check")
async def neuro_symbolic_check(req: NeuroSymbolicRequest):
    """
    Run DeepProbLog physics gatekeeper on a hypothesis dict.
    Returns {valid: bool, reason: str}.
    """
    valid, reason = run_deepproblog_check(req.hypothesis)
    return {"valid": valid, "reason": reason, "engine": "deepproblog+problog"}


# ── Route: Lean 4 auto-tactic generation via Qwen3-8B ────────────────────────

@app.post("/v1/lean4/suggest-tactic")
async def suggest_lean_tactic(body: dict):
    """Ask Qwen3-8B (thinking mode) to suggest a Lean 4 tactic for a proof obligation."""
    theorem = body.get("theorem", "")
    context = body.get("context", "")

    prompt = f"""You are a Lean 4 expert. Suggest a single tactic to close this proof obligation.
Respond with ONLY the tactic, no explanation.

Theorem: {theorem}
Context: {context}

Tactic:"""

    req = ChatRequest(
        messages=[{"role": "user", "content": prompt}],
        task_type="proof",
        temperature=0.2,
        max_tokens=128,
        enable_thinking=True,
        thinking_budget=2048,
    )
    return await chat_completions(req)


# ── Route: model info ─────────────────────────────────────────────────────────

@app.get("/v1/models")
async def list_models():
    return {
        "models": [
            {
                "id": QWEN3_MODEL,
                "task": "math/reasoning/proof/peer-review",
                "port": QWEN3_PORT,
                "thinking_mode": True,
                "quantization": QUANTIZATION or "fp16",
            },
            {
                "id": QWEN_CODER_MODEL,
                "task": "code/rust/synthesis",
                "port": QWEN_CODER_PORT,
                "thinking_mode": False,
                "quantization": QUANTIZATION or "fp16",
            },
            {
                "id": CODEBERT_MODEL,
                "task": "code-embeddings",
                "port": "in-process",
                "thinking_mode": False,
                "quantization": "none",
            },
        ],
        "neuro_symbolic": "deepproblog+problog",
        "gpu_type": GPU_TYPE,
    }


# ── Route: health ─────────────────────────────────────────────────────────────

@app.get("/health")
async def health():
    """Health check for Cloud Run / Vertex AI."""
    workers_ok = {}
    async with httpx.AsyncClient(timeout=5) as client:
        for name, port in [("qwen3", QWEN3_PORT), ("coder", QWEN_CODER_PORT)]:
            try:
                resp = await client.get(f"http://localhost:{port}/health")
                workers_ok[name] = resp.status_code == 200
            except Exception:
                workers_ok[name] = False

    all_ok = all(workers_ok.values())
    return {
        "status": "healthy" if all_ok else "degraded",
        "workers": workers_ok,
        "codebert_loaded": _codebert is not None,
        "gpu_type": GPU_TYPE,
        "quantization": QUANTIZATION or "fp16",
    }


# ── Route: metrics ────────────────────────────────────────────────────────────

@app.get("/metrics")
async def metrics():
    """Prometheus-compatible metrics stub."""
    return {
        "gpu_type": GPU_TYPE,
        "models_loaded": len(_vllm_procs),
        "codebert_loaded": _codebert is not None,
        "quant": QUANTIZATION or "fp16",
    }


# ── Entrypoint ────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    port = int(os.environ.get("PORT", 8080))
    uvicorn.run(app, host="0.0.0.0", port=port, log_level="info")
