import os
import json
from fastapi import FastAPI
from fastapi.staticfiles import StaticFiles
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from typing import List, Optional

app = FastAPI(title="rusty-SUNDIALS Visualization API")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

# Mount the static data directory to serve images/VTK files directly
DATA_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "data"))
app.mount("/static/data", StaticFiles(directory=DATA_DIR), name="data")

# In-memory storage of visualizations (could be replaced with sqlite later)
# We initialize it with the ITER fusion plasma simulation
DB_FILE = os.path.join(os.path.dirname(__file__), "storage.json")

def load_db():
    if os.path.exists(DB_FILE):
        with open(DB_FILE, "r") as f:
            return json.load(f)
    return {
        "visualizations": [
            {
                "id": "iter-disruption-v11",
                "title": "ITER Plasma Fusion Disruption",
                "description": "Simulation of 2D Reduced-MHD Thermal Quench and Tearing Modes, accelerated by rusty-SUNDIALS CVODE (BDF) and FLAGNO FGMRES Krylov solver. Captures electron temperature collapse and vacuum vessel eddy currents.",
                "tags": ["MHD", "Plasma", "ITER", "CVODE", "FGMRES"],
                "images": [
                    "/static/data/fusion/vtk_output/iter_disruption_hero.png",
                    "/static/data/fusion/vtk_output/iter_disruption_sequence.png",
                    "/static/data/fusion/vtk_output/iter_Te_midplane.png",
                    "/static/data/fusion/vtk_output/iter_jphi_midplane.png",
                    "/static/data/fusion/vtk_output/iter_psi_midplane.png"
                ],
                "dataset_path": "/static/data/fusion/rust_sim_output/"
            }
        ]
    }

def save_db(data):
    with open(DB_FILE, "w") as f:
        json.dump(data, f, indent=2)

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

if __name__ == "__main__":
    import uvicorn
    uvicorn.run("main:app", host="0.0.0.0", port=int(os.getenv("PORT", 8080)), reload=True)

# Mount the frontend React app last to handle all other routes
FRONTEND_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "mission-control", "dist"))
if os.path.exists(FRONTEND_DIR):
    app.mount("/", StaticFiles(directory=FRONTEND_DIR, html=True), name="frontend")
