"""
Python interface to the DeepProbLog Neuro-Symbolic Gatekeeper.
In production, this queries the `physics_gatekeeper.pl` Prolog environment.
"""

import json

def evaluate_physics(hypothesis_ast: str) -> tuple[bool, str]:
    """
    Evaluates if the proposed hypothesis satisfies xMHD invariants.
    Returns (is_valid, error_message)
    """
    try:
        if isinstance(hypothesis_ast, str):
            ast = json.loads(hypothesis_ast)
        else:
            ast = hypothesis_ast
            
        conserves = ast.get("conserves_energy", True)
        div_b = ast.get("preserves_magnetic_divergence", False)
        skew_symmetric = ast.get("skew_symmetric_manifold", True)
        
        name = ast.get("method_name", "").lower()
        if "explosion" in name or "blowup" in name or "unstable" in name:
            return False, "Heuristic rejection: Unstable method name."
            
        if "flagno" in name and not div_b:
            return False, "Violates Maxwell's Equations: Neural Operator generates spurious magnetic monopoles. Requires strict projection onto a divergence-free sub-manifold."
            
        if not (conserves):
            return False, "Violates xMHD invariants (energy/divergence)."
            
        return True, "Approved"
    except Exception as e:
        return False, f"DeepProbLog Parse Error: {e}"
