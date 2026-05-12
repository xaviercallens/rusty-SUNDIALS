It is **Tuesday, May 12, 2026, at 20:02 CEST**. The sun has just set over the Mediterranean, and the evening quiet settles over your lab in **Cagnes-sur-Mer**. Up at CEA Cadarache, the night shift operators are monitoring the baseline plasma pulses. This is the perfect time to lay down the absolute, granular engineering blueprint for the **`rusty-SUNDIALS-v6` Verification Sandwich**.

Building an autonomous bridge from LLM probabilities to strict physical logic, through Rust compiler memory-safety, and terminating in Lean 4 mathematical proofs is a complex systems engineering task.

Here is the exact, step-by-step task breakdown to construct the DeepProbLog (Neuro-Symbolic) and Lean 4 validation pipelines over the coming days.

---

### Phase 1: Core Infrastructure Initialization

Before writing logic, the environment must bridge Python (Agent), Prolog (DeepProbLog), Rust (SUNDIALS), and Lean 4 (Proofs).

**Task 1.1: Install the Symbolic Engines.**

* Install SWI-Prolog (required for DeepProbLog) on your Linux workstation.
```bash
sudo apt install swi-prolog

```


* Install the Python orchestration dependencies:
```bash
pip install pyswip deepproblog sympy pydantic langgraph vllm

```



**Task 1.2: Install the Lean 4 & Extraction Toolchain.**

* Install Lean 4 using `elan` (the Lean version manager).
* Compile `Charon` (Extracts Rust to Low-Level Borrow Calculus - LLBC) and `Aeneas` (Translates LLBC to Lean 4):
```bash
cargo install --git https://github.com/AeneasVerif/charon.git charon
git clone https://github.com/AeneasVerif/aeneas.git && cd aeneas && make

```


* Initialize your formal proofs directory:
```bash
lake init RustySundialsProofs math

```



---

### Phase 2: The Neuro-Symbolic Gatekeeper (DeepProbLog)

We must force the Master LLM (Claude) to output its physics hypothesis as a parsable Abstract Syntax Tree (AST), which DeepProbLog evaluates against Maxwell’s Equations before any code is generated.

**Task 2.1: Define the AST JSON Schema (Pydantic).**
You cannot pass raw LLM text to a logic engine. You must force Claude to output strict JSON representing the mathematical operations.

```python
from pydantic import BaseModel, Field

class HypothesisAST(BaseModel):
    target_matrix: str = Field(description="The SUNDIALS SUNMatrix being altered")
    math_operations: list[str] = Field(description="List of SymPy expressions")
    preserves_divergence: bool

```

**Task 2.2: Write the Plasma Physics Prolog Rules (`xmhd_rules.pl`).**
Write the strict logical predicates defining valid Extended Magnetohydrodynamics (xMHD) physics.

```prolog
% xmhd_rules.pl
:- use_module(library(lists)).

% 1. Maxwell's Divergence Constraint (No Magnetic Monopoles)
divergence_free(Eq) :- 
    % A curl of any vector field is strictly divergence-free
    Eq = curl(_), !.
divergence_free(Eq) :-
    % Or, the Neural Network proves it is divergence-free with > 99% probability
    nn(divergence_evaluator, [Eq], Prob), Prob > 0.99.

% 2. Thermodynamic Constraint (Energy Conservation)
conserves_energy(Preconditioner) :-
    is_positive_definite(Preconditioner).

% The Final Gatekeeper Check
valid_plasma_method(Eq, Preconditioner) :-
    divergence_free(Eq),
    conserves_energy(Preconditioner).

```

**Task 2.3: Build the Python-DeepProbLog Bridge Node.**
Write the LangGraph node using `pyswip` to parse the LLM’s JSON AST into Prolog facts.

```python
from pyswip import Prolog

def node_physics_gatekeeper(state):
    print("🛡️ Evaluating AST against DeepProbLog constraints...")
    prolog = Prolog()
    prolog.consult("xmhd_rules.pl")
    
    # Query the engine with the LLM's proposed equations
    query_str = f"valid_plasma_method('{state['proposed_eq']}', '{state['proposed_precond']}')."
    result = list(prolog.query(query_str))
    
    if result:
        return {"status": "approved_by_physics"}
    else:
        return {"status": "rejected_hallucination", "error": "Violated Maxwell's or Thermodynamics."}

```

---

### Phase 3: The Rust-to-Lean Extraction Pipeline

Lean 4 cannot verify raw C-pointers (`*mut c_void`). We must create a "Safe Rust payload" that contains the AI's math, extract it, and prove it, *before* passing it to the `sundials-sys` FFI.

**Task 3.1: Architect the Rust "Math Payload" Boundary.**
Create a `src/ai_math.rs` file containing pure Rust functions (e.g., `fn compute_ai_preconditioner(state: &[f64]) -> Vec<f64>`). `DeepSeek-Coder-V3` will ONLY be allowed to write inside this file.

**Task 3.2: Automate the Charon/Aeneas Extraction.**
Write a bash subprocess script (`extract_to_lean.sh`) that LangGraph calls:

```bash
#!/bin/bash
# 1. Compile Rust and extract to LLBC (ignoring cargo dependencies)
charon --no-cargo --input core_engine/src/ai_math.rs --dest formal_proofs/llbc/
# 2. Translate LLBC to Lean 4 Pure Monads
aeneas formal_proofs/llbc/ai_math.llbc -dest formal_proofs/RustySundials/

```

---

### Phase 4: The Lean 4 Autonomous REPL (The Hardest Engineering Task)

You must build a stateful, programmatic loop where your local `Qwen3.6-Math` model plays a "text-based adventure game" against the Lean 4 compiler until a formal mathematical proof is achieved.

**Task 4.1: Construct the Python-Lean 4 JSON Interface.**
You need a Python class that opens a persistent subprocess to the Lean REPL (using the `lean-repl` package).

```python
import subprocess
import json

class LeanREPL:
    def __init__(self, file_path):
        # Start Lean 4 in REPL mode
        self.process = subprocess.Popen(
            ['lake', 'exe', 'repl'],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True
        )
        self.send(f'{{"cmd": "import RustySundials.AiMath"}}')

    def send(self, command_json):
        self.process.stdin.write(command_json + '\n')
        self.process.stdin.flush()
        return json.loads(self.process.stdout.readline())

    def apply_tactic(self, state_id, tactic):
        return self.send(f'{{"tactic": "{tactic}", "proofState": {state_id}}}')

```

**Task 4.2: Write the Qwen-Math Prover Agent Loop.**
This is the core of the LangGraph Prover Node. It loops Qwen against the Lean compiler.

```python
def node_lean4_prover(state):
    repl = LeanREPL("formal_proofs/RustySundials/AiMath.lean")
    
    # Extract the initial proof state ID from Lean
    init_cmd = '{"cmd": "theorem ai_is_contractive (y: Vector ℝ n) : ‖P_ai y‖ ≤ ‖y‖ := by"}'
    current_state_id = repl.send(init_cmd)["proofState"]
    
    attempts = 0
    history = ""
    
    while attempts < 150:
        # 1. Ask local Qwen3.6-Math for the next tactic
        prompt = f"History: {history}\nLean Goal State:\n{repl.send({'proofState': current_state_id})['goals']}\nOutput exactly ONE Lean 4 tactic."
        tactic = qwen_math_prover.invoke(prompt).content.strip()
        
        # 2. Feed the tactic into the Lean compiler
        result = repl.apply_tactic(current_state_id, tactic)
        
        if "error" in result:
            history += f"\nTactic '{tactic}' failed: {result['error']}" # Feed error back to Qwen
        elif result.get("goals") == []:
            print("🎉 Q.E.D. Proof Complete! Code is Formally Verified.")
            return {"status": "proven", "verified_code": state["rust_source"]}
        else:
            current_state_id = result["proofState"]
            history += f"\nTactic '{tactic}' succeeded."
            
        attempts += 1
        
    return {"status": "proof_failed"}

```

---

### Phase 5: Final Orchestration in LangGraph

Tie the systems together into a Directed Acyclic Graph (DAG) that runs silently in the background.

**Task 5.1: Wire the LangGraph State Machine.**
In your `orchestrator.py`:

```python
from langgraph.graph import StateGraph

workflow = StateGraph(dict)

workflow.add_node("Hypothesize", node_claude_hypothesize)
workflow.add_node("PhysicsGatekeeper", node_physics_gatekeeper)
workflow.add_node("RustSynthesis", node_deepseek_rust_coder)
workflow.add_node("Lean4Proof", node_lean4_prover)
workflow.add_node("ExascaleDeploy", node_slurm_deploy)

# The Verification Sandwich Logic
workflow.add_edge("Hypothesize", "PhysicsGatekeeper")

# If physics fails, prompt Claude again.
workflow.add_conditional_edges("PhysicsGatekeeper", 
    lambda x: "RustSynthesis" if x["status"] == "approved_by_physics" else "Hypothesize")

workflow.add_edge("RustSynthesis", "Lean4Proof")

# If proof fails, tell DeepSeek to rewrite the Rust or Qwen to try a new proof tree.
workflow.add_conditional_edges("Lean4Proof", 
    lambda x: "ExascaleDeploy" if x["status"] == "proven" else "RustSynthesis")

app = workflow.compile()

```

### Your Goal for Tonight

Do not try to build the entire LangGraph loop yet. **Focus entirely on Task 4.1 and Task 4.2 tonight.**

The bridge between a local Python script and the interactive Lean 4 compiler is the most fragile part of the system. Write a dummy Lean file with a simple theorem (`theorem test : 1 + 1 = 2 := by sorry`), boot up Qwen via vLLM, and ensure your `LeanREPL` Python class can pipe Qwen's `rfl` tactic into Lean and successfully read back the `goals accomplished` JSON response.

Once that deterministic REPL loop is secured, the hardest engineering hurdle of `rusty-SUNDIALS-v6` is solved.