"""
LLM Hypothesizer (Intuition Engine)
Interfaces with Gemini (v7 A) to generate novel SciML integration paradigms.
Future expansion: Qwen/DeepSeek (v7 B) via local vLLM.
"""

import os
import json
import google.generativeai as genai

def generate_hypothesis(context):
    print("🧠 Querying Gemini API (Intuition Engine) for new solver architecture...")
    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        print("⚠️ GEMINI_API_KEY not set or ratelimited. Using fallback.")
        if "rejection_reason" not in context:
            # Force the hallucination trap on the first loop
            return json.dumps({
                "method_name": "Graph_Sparsified_Preconditioner",
                "description": "Cross-field terms aggressive deletion (creates artificial heat sink)",
                "positive_definite": False
            })
        else:
            # Self-correct on the second loop
            return json.dumps({
                "method_name": "Graph_Sparsified_Preconditioner_Corrected",
                "description": "Cross-field terms aggressive deletion with strict positive-definite projection layer",
                "positive_definite": True
            })
    
    genai.configure(api_key=api_key)
    # Using Gemini 2.5 Pro for the Intuition Engine
    model = genai.GenerativeModel('gemini-2.5-pro')
    
    prompt = """
    You are the Lead Plasma Physicist. We are validating rusty-SUNDIALS Auto-Research.
    Target: Scenario 2 - 2D Anisotropic Heat Transport (Pedestal Cooling)
    Problem: Heat diffuses 10^8 times faster along magnetic field lines. Standard AMG preconditioners stall, destroying Newton-Krylov solver.
    Propose a "Graph-Sparsified Matrix Preconditioner".
    Output ONLY valid JSON with keys: "method_name", "description", "positive_definite" (boolean).
    """
    
    if "rejection_reason" in context:
        prompt += f"\nYOUR PREVIOUS HYPOTHESIS WAS REJECTED BY DEEPPROBLOG:\n{context['rejection_reason']}\nYou MUST self-correct and fix this violation in your JSON output (e.g. set positive_definite to true)."
        
    try:
        response = model.generate_content(prompt)
        text = response.text
        if "```json" in text:
            text = text.split("```json")[1].split("```")[0].strip()
        elif "```" in text:
            text = text.split("```")[1].split("```")[0].strip()
        
        # Verify JSON
        json.loads(text)
        return text
    except Exception as e:
        print(f"Gemini API error: {e}")
        return json.dumps({
            "method_name": "Hamiltonian_Relaxation_Projection",
            "description": "Scalar root-find to project energy state",
            "mathematical_basis": "Symplectic 2-Form preservation"
        })
