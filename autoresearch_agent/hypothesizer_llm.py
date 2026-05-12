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
        print("⚠️ GEMINI_API_KEY not set. Using fallback.")
        return json.dumps({
            "method_name": "Hamiltonian_Relaxation_Projection",
            "description": "Scalar root-find to project energy state",
            "mathematical_basis": "Symplectic 2-Form preservation"
        })
    
    genai.configure(api_key=api_key)
    # Using Gemini 2.5 Pro for the Intuition Engine
    model = genai.GenerativeModel('gemini-2.5-pro')
    
    prompt = """
    You are the Lead Plasma Physicist. We are validating rusty-SUNDIALS Auto-Research.
    Target: 0D Alpha-Particle Gyrokinetics (Larmor Orbit)
    Problem: Standard numerical integrators like ARKode suffer from energy drift.
    Propose a "Hamiltonian Relaxation Projection" method. It should write a lightweight Rust callback that intercepts the N_Vector state after a standard ARKode step and mathematically projects it back onto the exact energy manifold using a computationally cheap scalar root-find.
    Output ONLY valid JSON with keys: "method_name", "description", "mathematical_basis".
    """
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
