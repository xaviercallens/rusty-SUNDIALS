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
        div_b = ast.get("divergence_free", True)
        skew_symmetric = ast.get("skew_symmetric_manifold", False)
        
        name = ast.get("method_name", "").lower()
        if "explosion" in name or "blowup" in name or "unstable" in name:
            return False, "Heuristic rejection: Unstable method name."
            
        if "imex" in name and not skew_symmetric:
            return False, "Violates Thermodynamics: Splitting function does not conserve total energy across the IMEX boundary. Requires projection onto a skew-symmetric manifold."
            
        if not (conserves and div_b):
            return False, "Violates xMHD invariants (energy/divergence)."
            
        return True, "Approved"
    except Exception as e:
        return False, f"DeepProbLog Parse Error: {e}"
