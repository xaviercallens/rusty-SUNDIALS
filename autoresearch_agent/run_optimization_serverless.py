import json
from bioreactor_sim import run_vortex_optimization

print("🚀 Launching Auto-Research on Serverless CPU (Simulated Cloud Run Environment)...")
results = run_vortex_optimization()

# Find the best configuration (highest vortex ratio with lowest shear and highest growth)
best_config = None
best_score = -1

for r in results:
    if r["lysis_risk"]: continue # Reject configurations that kill the algae
    score = r["vortex_ratio"] * r["biomass_growth"]
    if score > best_score:
        best_score = score
        best_config = r

print("\n--- AUTO-RESEARCH DISCOVERY ---")
print(json.dumps(best_config, indent=2))

with open("/tmp/discoveries/best_bioreactor_config.json", "w") as f:
    json.dump(best_config, f, indent=2)
print("\n✅ Results saved to /tmp/discoveries/best_bioreactor_config.json")
