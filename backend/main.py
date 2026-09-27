import os
import json
import time
import math
from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import List, Optional

app = FastAPI(title="rusty-SUNDIALS Mission Control API — v17")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

# Mount the static data directory to serve images/VTK files directly
DATA_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "data"))
if os.path.exists(DATA_DIR):
    app.mount("/static/data", StaticFiles(directory=DATA_DIR), name="data")

# Serve paper figures directly
FIGURES_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "paper", "figures"))
if os.path.exists(FIGURES_DIR):
    app.mount("/static/figures", StaticFiles(directory=FIGURES_DIR), name="figures")

DB_FILE = os.path.join(os.path.dirname(__file__), "storage.json")

def load_db():
    if os.path.exists(DB_FILE):
        with open(DB_FILE, "r") as f:
            return json.load(f)
    return {
        "visualizations": [
            {
                "id": "iter-disruption-2d",
                "title": "ITER 2D Reduced-MHD Disruption",
                "description": "2D proxy model (168K DOF) — thermal quench, vessel eddy currents, m=2 tearing mode.",
                "tags": ["MHD", "Plasma", "ITER", "CVODE", "2D"],
                "images": [
                    "/static/data/fusion/vtk_output/iter_disruption_hero.png",
                    "/static/data/fusion/vtk_output/iter_disruption_sequence.png",
                    "/static/data/fusion/vtk_output/iter_disruption_3d_torus.png"
                ],
                "dataset_path": "/static/data/fusion/rust_sim_output/"
            },
            {
                "id": "iter-disruption-3d",
                "title": "ITER 3D Toroidal Disruption",
                "description": "3D toroidal extension (672K DOF) — 16 toroidal slices with n=1 helical tearing mode coupling cos(2θ-φ).",
                "tags": ["MHD", "Plasma", "ITER", "3D", "Toroidal", "n=1"],
                "images": [
                    "/static/data/fusion/vtk_output_3d/iter_3d_torus_video.mp4",
                    "/static/data/fusion/vtk_output_3d/iter_3d_torus_hero.png",
                    "/static/data/fusion/vtk_output_3d/iter_3d_all_slices.png",
                    "/static/data/fusion/vtk_output_3d/iter_3d_temporal_sequence.png",
                    "/static/data/fusion/vtk_output_3d/iter_3d_cross_phi0.png",
                    "/static/data/fusion/vtk_output_3d/iter_3d_cross_phi180.png"
                ],
                "dataset_path": "/static/data/fusion/rust_sim_output_3d/"
            }
        ],
        "auto_research": [],
        "benchmarks": []
    }

def save_db(data):
    with open(DB_FILE, "w") as f:
        json.dump(data, f, indent=2)


# ═══════════════════════════════════════════════════════════════
# Visualizations API
# ═══════════════════════════════════════════════════════════════
@app.get("/api/visualizations")
def get_visualizations():
    db = load_db()
    return db["visualizations"]

class VisualizationCreate(BaseModel):
    id: str
    title: str
    description: str
    tags: List[str] = []
    images: List[str] = []
    dataset_path: Optional[str] = None

@app.post("/api/visualizations")
def create_visualization(viz: VisualizationCreate):
    db = load_db()
    db["visualizations"].append(viz.dict())
    save_db(db)
    return viz


# ═══════════════════════════════════════════════════════════════
# Datasets API
# ═══════════════════════════════════════════════════════════════
@app.get("/api/datasets")
def get_datasets():
    datasets = []
    # 2D dataset
    path_2d = os.path.join(DATA_DIR, "fusion", "rust_sim_output")
    if os.path.exists(path_2d):
        files_2d = [f for f in os.listdir(path_2d) if f.endswith('.csv')]
        datasets.append({
            "id": "iter-2d-168k",
            "name": "ITER 2D Proxy Model (168K DOF)",
            # AUDIT 2026-09-27: examples/iter_disruption.rs integrates a
            # y-independent RHS (prescribed closed form); not an MHD solve.
            "provenance": "prescribed analytic trajectory, not a physics simulation",
            "files": len(files_2d),
            "path": "/static/data/fusion/rust_sim_output/",
            "dof": 168000,
            "grid": "200×400 (ρ,θ)"
        })
    # 3D dataset
    path_3d = os.path.join(DATA_DIR, "fusion", "rust_sim_output_3d")
    if os.path.exists(path_3d):
        files_3d = [f for f in os.listdir(path_3d) if f.endswith('.csv')]
        datasets.append({
            "id": "iter-3d-672k",
            "name": "ITER 3D Toroidal (672K DOF)",
            # AUDIT 2026-09-27: examples/iter_disruption_3d.rs evaluates a
            # closed form directly; no ODE solve.
            "provenance": "closed-form evaluation, no solver, not a physics simulation",
            "files": len(files_3d),
            "path": "/static/data/fusion/rust_sim_output_3d/",
            "dof": 672000,
            "grid": "100×200×16 (ρ,θ,φ)"
        })
    return datasets


# ═══════════════════════════════════════════════════════════════
# Auto-Research API
# ═══════════════════════════════════════════════════════════════
class AutoResearchResult(BaseModel):
    id: str
    name: str
    status: str  # "running", "completed", "failed"
    findings: Optional[dict] = None
    timestamp: Optional[str] = None

# AUDIT 2026-09-27 (docs/audit/fusion-2026-09-27/README.md): the three
# "auto-research" results below were never measured. Earlier versions of the
# POST endpoints returned hardcoded dicts / `if step < 8` schedules (H100,
# cuSPARSE 8.3 ms, FP8 0.9 ms, 157.8x, 45k-param MPNN, 2.5 GPU-h ...) with
# "status": "completed" and persisted them to storage.json. storage.json is
# left untouched (data owned by the author); instead, those entries are
# relabelled when served.
SIMULATED_AUTO_RESEARCH_IDS = {
    "gpu-ablation-v1",
    "adaptive-precision-v1",
    "arch-comparison-v1",
}
NOT_MEASURED_NOTE = (
    "NOT MEASURED — demo placeholder. These values were hardcoded in "
    "backend/main.py, not produced by any benchmark run. See "
    "docs/audit/fusion-2026-09-27/RETRACTION_NOTICE.md."
)


def _relabel_simulated(entry: dict) -> dict:
    """Mark a stored auto-research entry as not measured if it is one of the
    hardcoded ones; other entries are returned unchanged."""
    if entry.get("id") in SIMULATED_AUTO_RESEARCH_IDS:
        relabelled = dict(entry)
        relabelled["status"] = "not_measured"
        relabelled["audit_note"] = NOT_MEASURED_NOTE
        return relabelled
    return entry


@app.get("/api/auto-research")
def get_auto_research():
    db = load_db()
    return [_relabel_simulated(e) for e in db.get("auto_research", [])]

@app.post("/api/auto-research")
def submit_auto_research(result: AutoResearchResult):
    db = load_db()
    if "auto_research" not in db:
        db["auto_research"] = []
    db["auto_research"].append(result.dict())
    save_db(db)
    return result

@app.post("/api/auto-research/run-gpu-ablation")
def run_gpu_ablation():
    """GPU ablation (GNN-FP8 vs cuSPARSE ILU0 vs CPU ILU): NOT IMPLEMENTED.

    AUDIT 2026-09-27: this endpoint used to return hardcoded timings
    (142 / 8.3 / 2.1 / 0.9 ms, 157.8x on an "H100") as "completed" and
    persist them. No such benchmark exists in the repository. It now returns
    an explicit not-measured placeholder with the same keys and does not
    write to storage.json.
    """
    methods = [
        "CPU Sparse ILU-GMRES",
        "GPU cuSPARSE ILU0-GMRES",
        "Neural-FGMRES FP16",
        "Neural-FGMRES FP8 (E4M3)",
    ]
    return {
        "id": "gpu-ablation-v1",
        "name": "GPU-Native Baseline Ablation",
        "status": "not_measured",
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "audit_note": NOT_MEASURED_NOTE,
        "findings": {
            "description": "NOT MEASURED — no GPU ablation benchmark exists in this repository.",
            "dof": None,
            "hardware": None,
            "benchmarks": [
                {"method": m, "hardware": None, "time_ms": None, "speedup": "NOT MEASURED"}
                for m in methods
            ],
            "analysis": {
                "hardware_contribution": "NOT MEASURED",
                "algorithmic_contribution": "NOT MEASURED",
                "total_speedup_decomposition": "NOT MEASURED — demo placeholder",
            },
        },
    }

@app.post("/api/auto-research/run-adaptive-precision")
def run_adaptive_precision():
    """Eisenstat-Walker adaptive precision experiment: NOT IMPLEMENTED.

    AUDIT 2026-09-27: this endpoint used to generate its "trajectories" from a
    hardcoded schedule (`if step < 8: FP8 ... elif step < 15: FP16 ...`, with
    residuals `tol * exp(-0.8 * step)`) and return them as a "completed"
    experiment. No Newton solve and no precision switching took place. It now
    returns an explicit not-measured placeholder with the same keys and does
    not write to storage.json.
    """
    return {
        "id": "adaptive-precision-v1",
        "name": "Adaptive Eisenstat-Walker Precision Forcing",
        "status": "not_measured",
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "audit_note": NOT_MEASURED_NOTE,
        "findings": {
            "description": "NOT MEASURED — no adaptive-precision solver run exists in this repository.",
            "fixed_fp8": {"total_newton_iters": None, "final_residual": None},
            "adaptive": {"total_newton_iters": None, "final_residual": None},
            "improvement": "NOT MEASURED — demo placeholder",
            "trajectory_fixed": [],
            "trajectory_adaptive": [],
        },
    }

@app.post("/api/auto-research/run-architecture-comparison")
def run_architecture_comparison():
    """Neural preconditioner comparison (MPNN vs FNO vs DeepONet): NOT IMPLEMENTED.

    AUDIT 2026-09-27: this endpoint used to return a hardcoded table
    (parameter counts, 0.9/1.2/1.5 ms inference, Krylov iterations, GPU-hours)
    as a "completed" comparison. No network was trained or evaluated, and
    data/gnn_weights/ does not exist. It now returns an explicit not-measured
    placeholder with the same keys and does not write to storage.json.
    """
    names = ["MPNN (3-layer)", "FNO (4-mode, 3-layer)", "DeepONet (branch-trunk)"]
    return {
        "id": "arch-comparison-v1",
        "name": "Alternative Neural Preconditioner Architectures",
        "status": "not_measured",
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "audit_note": NOT_MEASURED_NOTE,
        "findings": {
            "description": "NOT MEASURED — no neural preconditioner has been trained or benchmarked in this repository.",
            "architectures": [
                {
                    "name": n,
                    "params": None,
                    "inference_ms": None,
                    "krylov_iters_to_converge": None,
                    "training_gpu_hours": None,
                    "strengths": None,
                    "weaknesses": None,
                }
                for n in names
            ],
            "recommendation": "NOT MEASURED — demo placeholder",
        },
    }


# ═══════════════════════════════════════════════════════════════
# Benchmarks API
# ═══════════════════════════════════════════════════════════════
@app.get("/api/benchmarks")
def get_benchmarks():
    """AUDIT 2026-09-27: every value here used to be a literal (C 150 ms,
    Rust 142 ms, FP8 0.9 ms, 157.8x, relative costs) presented as a benchmark
    result and quoted as measured in paper/manuscript_v17.md. None was
    produced by a run. Keys are kept (values nulled) so clients do not break.
    """
    return {
        "status": "not_measured",
        "audit_note": NOT_MEASURED_NOTE,
        "c_vs_rust": {
            "c_sundials_sparse_ilu_ms": None,
            "rust_sundials_sparse_ilu_ms": None,
            "rust_neural_fgmres_fp8_ms": None,
            "parity_ratio": None,
            "speedup_neural": None,
        },
        "gpu_ablation": {
            "cpu_sparse_ilu_ms": None,
            "gpu_cusparse_ilu0_ms": None,
            "gpu_neural_fp16_ms": None,
            "gpu_neural_fp8_ms": None,
            "hardware_speedup": "NOT MEASURED",
            "algorithmic_speedup": "NOT MEASURED",
            "total_speedup": "NOT MEASURED",
        },
        "relative_cost": {
            "v100_baseline": None,
            "cloud_build_cpu": None,
            "h100_tensor_core": None,
        },
    }


# ═══════════════════════════════════════════════════════════════
# Peer Review POC
# ═══════════════════════════════════════════════════════════════
@app.post("/api/peer_review/poc")
def trigger_poc():
    import subprocess
    script_path = os.path.join(os.path.dirname(__file__), "..", "scripts", "reproduce_v12_poc.py")
    if os.path.exists(script_path):
        subprocess.run(["python3", script_path], check=True)
    poc_file = os.path.join(DATA_DIR, "fusion", "poc_output", "v12_poc_results.json")
    if os.path.exists(poc_file):
        with open(poc_file, "r") as f:
            data = json.load(f)
        # AUDIT 2026-09-27: reproduce_v12_poc.py computes these curves from
        # closed-form expressions (cpu=(dof/1000)**2*0.05, fp8_res*=0.78 then
        # *0.95 + sin(i)*1e-4). No solver, PCIe transfer or GPU is involved.
        # Label the payload (keys kept so the Reproducibility page still renders).
        data["synthetic"] = True
        data["status"] = "synthetic_formula_output_not_a_benchmark"
        data["audit_note"] = (
            "SYNTHETIC — generated from hand-written formulas in "
            "scripts/reproduce_v12_poc.py, not measured. See "
            "docs/audit/fusion-2026-09-27/RETRACTION_NOTICE.md."
        )
        return data
    return {"status": "poc_not_available"}


# ═══════════════════════════════════════════════════════════════
# Health Check
# ═══════════════════════════════════════════════════════════════
@app.get("/api/health")
def health():
    return {
        "status": "ok",
        "version": "v17",
        "solver": "rusty-SUNDIALS",
        "dof_2d": 168000,
        "dof_3d": 672000
    }


from fastapi.responses import FileResponse
from fastapi import Request

# Mount the frontend React app at root
FRONTEND_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "mission-control", "dist"))
if os.path.exists(FRONTEND_DIR):
    app.mount("/", StaticFiles(directory=FRONTEND_DIR, html=True), name="frontend")

@app.exception_handler(404)
async def custom_404_handler(request: Request, exc):
    if not request.url.path.startswith("/api/") and os.path.exists(os.path.join(FRONTEND_DIR, "index.html")):
        return FileResponse(os.path.join(FRONTEND_DIR, "index.html"))
    from fastapi.responses import JSONResponse
    return JSONResponse(status_code=404, content={"detail": "Not Found"})

if __name__ == "__main__":
    import uvicorn
    uvicorn.run("main:app", host="0.0.0.0", port=int(os.getenv("PORT", 8080)), reload=True)
