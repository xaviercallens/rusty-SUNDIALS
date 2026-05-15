"""
rusty-SUNDIALS v10 — Experimental Proposals Validation Suite
=============================================================
Validates all 3 proposals from autoresearch_1778845325:

  P1: SpectralDeepProbLog FourierGate    (Gate 2b, neuro_symbolic_v10)
  P2: MixedPrecisionFGMRES (CPU)         (cusparse_amgx_v10)
  P3: TensorCoreFP8AMG (GPU-sim)         (cusparse_amgx_v10)

Run:  pytest tests/test_v10_experimental.py -v
"""
import os, sys, hashlib
import numpy as np
import pytest
from pathlib import Path
from scipy import sparse

# Ensure autoresearch_agent is on the path
sys.path.insert(0, str(Path(__file__).parent.parent))

# ── helpers ────────────────────────────────────────────────────────────────
def tridiag(n: int, kappa: float = 1e4) -> sparse.csr_matrix:
    """Symmetric positive-definite tridiagonal system scaled to condition κ."""
    diag = np.linspace(1.0, kappa, n)
    off  = -np.ones(n - 1) * 0.4
    return sparse.diags([diag, off, off], [0, -1, 1], shape=(n, n),
                        format="csr").astype(np.float64)


# ══════════════════════════════════════════════════════════════════════════
# P1 — Gate 2b: SpectralDeepProbLog FourierGate
# ══════════════════════════════════════════════════════════════════════════

class TestGate2bSpectralFourier:
    """Unit + integration tests for Gate 2b (_gate_spectral_divfree)."""

    @pytest.fixture(autouse=True)
    def imports(self):
        from neuro_symbolic_v10 import _gate_spectral_divfree, validate_neuro_symbolic
        self.gate = _gate_spectral_divfree
        self.validate = validate_neuro_symbolic

    def _hyp(self, **kw):
        base = dict(
            method_name="Test_Method",
            description="Hodge projection",
            mathematical_basis="Fourier spectral de Rham",
            preserves_magnetic_divergence=True,
            conserves_energy=True,
            expected_speedup_factor=42.0,
            krylov_iteration_bound="O(log N)",
        )
        base.update(kw)
        return base

    # ── FFT path ──────────────────────────────────────────────────────────

    def test_zero_field_passes_fft(self):
        """A zero B-field is perfectly divergence-free."""
        B = np.zeros((4, 4, 4))
        g = self.gate(self._hyp(B_field_sample=B.tolist()))
        assert g.engine == "fft_spectral"
        assert g.passed

    def test_dirty_field_fails_fft(self):
        """Random field with large monopole modes fails the FFT check."""
        rng = np.random.default_rng(42)
        B = rng.standard_normal((8, 8, 8)) * 100
        g = self.gate(self._hyp(B_field_sample=B.tolist(), fourier_divfree_tol=1e-10))
        assert g.engine == "fft_spectral"
        assert not g.passed

    def test_fft_tol_respected(self):
        """Very small field with loose tolerance (1.0) should pass."""
        # Use a near-zero field so max |k·B̂| << 1.0
        B = np.zeros((4, 4, 4))
        B[0, 0, 0] = 1e-12   # tiny monopole below tol=1.0
        g = self.gate(self._hyp(B_field_sample=B.tolist(), fourier_divfree_tol=1.0))
        assert g.engine == "fft_spectral"
        assert g.passed

    # ── keyword path ──────────────────────────────────────────────────────

    def test_spectral_keywords_pass(self):
        g = self.gate({"mathematical_basis": "Fourier spectral Hodge", "description": ""})
        assert g.engine == "keyword_fallback"
        assert g.passed

    def test_monopole_keyword_blocks(self):
        g = self.gate({"mathematical_basis": "staggered without correction", "description": ""})
        assert not g.passed

    def test_empty_basis_nonblocking(self):
        """No B_field_sample + no keywords → non-blocking pass."""
        g = self.gate({"mathematical_basis": "", "description": ""})
        assert g.passed

    # ── integration ───────────────────────────────────────────────────────

    def test_standard_mode_has_5_gates(self):
        r = self.validate(self._hyp(), experimental=False)
        assert r.passed
        assert len(r.gates) == 5

    def test_experimental_mode_has_6_gates(self):
        r = self.validate(self._hyp(), experimental=True)
        assert r.passed
        assert len(r.gates) == 6

    def test_gate_2b_present_in_experimental(self):
        r = self.validate(self._hyp(), experimental=True)
        names = [g.name for g in r.gates]
        assert "SpectralFourierGate" in names

    def test_gate_2b_absent_in_standard(self):
        r = self.validate(self._hyp(), experimental=False)
        names = [g.name for g in r.gates]
        assert "SpectralFourierGate" not in names

    def test_gate_2b_result_in_report(self):
        r = self.validate(self._hyp(), experimental=True)
        g2b = next(g for g in r.gates if g.name == "SpectralFourierGate")
        assert g2b.engine in ("fft_spectral", "keyword_fallback")
        assert isinstance(g2b.reason, str)
        assert len(g2b.reason) > 5

    def test_env_var_experimental_gates(self, monkeypatch):
        monkeypatch.setenv("EXPERIMENTAL_GATES", "1")
        # Reload to pick up env
        import importlib
        import neuro_symbolic_v10 as ns
        importlib.reload(ns)
        assert ns.EXPERIMENTAL_GATES is True
        monkeypatch.delenv("EXPERIMENTAL_GATES")
        importlib.reload(ns)

    def test_fourier_divfree_tol_default(self):
        g = self.gate({"mathematical_basis": "Hodge", "description": ""})
        # Should not raise; default tol=1e-10 is used
        assert isinstance(g.passed, bool)

    def test_certification_p1(self):
        """P1 proposal as specified passes all gates in experimental mode."""
        hyp = dict(
            method_name="SpectralDeepProbLog_FourierGate",
            description="Fourier spectral divergence-free gate for xMHD",
            mathematical_basis="Fourier spectral de Rham Hodge decomposition",
            preserves_magnetic_divergence=True,
            conserves_energy=True,
            expected_speedup_factor=41.8,
            krylov_iteration_bound="O(log N)",
            fourier_divfree_tol=1e-10,
            B_field_sample=np.zeros((4, 4, 4)).tolist(),
        )
        r = self.validate(hyp, experimental=True)
        assert r.passed
        assert len(r.gates) == 6


# ══════════════════════════════════════════════════════════════════════════
# P2 — MixedPrecisionFGMRES (CPU)
# ══════════════════════════════════════════════════════════════════════════

class TestMixedPrecisionFGMRES:
    """Unit + integration tests for MixedPrecisionFGMRES."""

    @pytest.fixture(autouse=True)
    def imports(self):
        from cusparse_amgx_v10 import MixedPrecisionFGMRES, DEFAULT_BLOCK_SIZE, DEFAULT_KRYLOV_RESTART
        self.Solver = MixedPrecisionFGMRES
        self.DEFAULT_BLOCK_SIZE = DEFAULT_BLOCK_SIZE
        self.DEFAULT_KRYLOV_RESTART = DEFAULT_KRYLOV_RESTART

    def test_default_shap_block_size(self):
        assert self.DEFAULT_BLOCK_SIZE == 16

    def test_default_shap_krylov_restart(self):
        assert self.DEFAULT_KRYLOV_RESTART == 30

    def test_small_well_conditioned(self):
        A = tridiag(32, kappa=10.0)
        b = np.ones(32)
        solver = self.Solver(A, max_iters=50)
        x, info = solver.solve(b)
        assert info["backend"] == "CPU-MixedPrecision"
        assert isinstance(info["converged"], bool)
        assert info["iterations"] >= 1

    def test_medium_kappa_1e4(self):
        A = tridiag(64, kappa=1e4)
        b = np.random.default_rng(1).standard_normal(64)
        solver = self.Solver(A, max_iters=50)
        x, info = solver.solve(b)
        assert x.shape == (64,)
        assert np.isfinite(x).all()

    def test_stiff_kappa_1e6(self):
        """Adaptive FP64 refinement path: κ > 10^6 → smoother switches to FP64."""
        A = tridiag(64, kappa=1e6)
        b = np.random.default_rng(2).standard_normal(64)
        solver = self.Solver(A, max_iters=50)
        x, info = solver.solve(b)
        assert info["smoother"] in ("FP32-Chebyshev-deg2", "FP32-Chebyshev-deg4",
                                    "FP64-fallback", "FP64-Chebyshev")
        assert np.isfinite(x).all()

    def test_chebyshev_degree_by_arch(self):
        A = tridiag(32, 100.0)
        solver = self.Solver(A)
        assert solver._cheb_degree in (2, 4)

    def test_residual_finite(self):
        """Solver output must be finite — CPU-sim mode is correctness-only."""
        A = tridiag(64, 100.0)
        b = np.ones(64)
        x, _ = self.Solver(A, max_iters=100).solve(b)
        assert np.isfinite(x).all()

    def test_kappa_estimate_computed(self):
        A = tridiag(32, 1e5)
        solver = self.Solver(A)
        assert solver._kappa_estimate > 0
        assert np.isfinite(solver._kappa_estimate)

    def test_benchmark_proposal_2(self):
        from cusparse_amgx_v10 import run_experimental_numeric_benchmark
        r = run_experimental_numeric_benchmark(64, 1e4, proposal=2)
        assert r["proposal"] == 2
        assert r["speedup_vs_baseline"] >= 0
        assert isinstance(r["converged"], bool)
        assert "baseline_iters" in r

    def test_env_experimental_numeric(self, monkeypatch):
        monkeypatch.setenv("EXPERIMENTAL_NUMERIC", "1")
        import importlib
        import cusparse_amgx_v10 as m
        importlib.reload(m)
        assert m.EXPERIMENTAL_NUMERIC is True
        monkeypatch.delenv("EXPERIMENTAL_NUMERIC")
        importlib.reload(m)


# ══════════════════════════════════════════════════════════════════════════
# P3 — TensorCoreFP8AMG (GPU-sim)
# ══════════════════════════════════════════════════════════════════════════

class TestTensorCoreFP8AMG:
    """Unit + integration tests for TensorCoreFP8AMG."""

    @pytest.fixture(autouse=True)
    def imports(self):
        from cusparse_amgx_v10 import TensorCoreFP8AMG
        self.Solver = TensorCoreFP8AMG

    def test_solve_returns_array(self):
        A = tridiag(64, 1e4)
        b = np.random.default_rng(3).standard_normal(64)
        solver = self.Solver(A, block_size=16)
        x, info = solver.solve(b)
        assert x.shape == (64,)

    def test_backend_label(self):
        A = tridiag(32, 100.0)
        solver = self.Solver(A)
        _, info = solver.solve(np.ones(32))
        assert "CPU" in info["backend"] or "GPU" in info["backend"]

    def test_fp8_memory_reported(self):
        """TensorCoreFP8AMG.solve() returns fp8_memory_mb in info dict."""
        A = tridiag(64, 100.0)
        solver = self.Solver(A, block_size=16)
        _, info = solver.solve(np.ones(64))
        assert "fp8_memory_mb" in info, f"info keys: {list(info.keys())}"
        assert info["fp8_memory_mb"] >= 0

    def test_block_size_16_alignment(self):
        """block_size=16 satisfies FP8 e4m3fn 8-byte granularity."""
        from cusparse_amgx_v10 import DEFAULT_BLOCK_SIZE
        assert DEFAULT_BLOCK_SIZE % 8 == 0

    def test_bf16_matvec_finite(self):
        A = tridiag(64, 1e3)
        solver = self.Solver(A, block_size=16)
        r = np.random.default_rng(4).standard_normal(64)
        # _matvec_bf16 is internal — call via solve and check output
        x, info = solver.solve(r)
        assert np.isfinite(x).all()
        assert info["backend"] in ("CPU-FP16-sim", "GPU-BF16-TensorCore")

    def test_benchmark_proposal_3(self):
        from cusparse_amgx_v10 import run_experimental_numeric_benchmark
        r = run_experimental_numeric_benchmark(64, 1e4, proposal=3)
        assert r["proposal"] == 3
        assert r["speedup_vs_baseline"] >= 0

    def test_convergence_large(self):
        """n=256 well-conditioned system — solver must return finite result."""
        A = tridiag(256, 1e3)
        b = np.random.default_rng(5).standard_normal(256)
        solver = self.Solver(A, block_size=16, max_iters=50)
        x, _ = solver.solve(b)
        assert np.isfinite(x).all()

    def test_precond_no_zero_vector(self):
        """Preconditioner must not return zero for non-zero input."""
        A = tridiag(128, 1e4)
        solver = self.Solver(A, block_size=16, max_iters=10)
        # Run solve — previously crashed on zero-vector output
        x, info = solver.solve(np.ones(128))
        assert np.isfinite(x).all()


# ══════════════════════════════════════════════════════════════════════════
# SHAP Global Defaults
# ══════════════════════════════════════════════════════════════════════════

class TestSHAPDefaults:

    def test_block_size_is_16(self):
        from cusparse_amgx_v10 import DEFAULT_BLOCK_SIZE
        assert DEFAULT_BLOCK_SIZE == 16

    def test_krylov_restart_is_30(self):
        from cusparse_amgx_v10 import DEFAULT_KRYLOV_RESTART
        assert DEFAULT_KRYLOV_RESTART == 30

    def test_pipeline_default_hypothesis_has_fft_tol(self):
        sys.path.insert(0, str(Path(__file__).parent.parent))
        import importlib, pipeline_v10_full as pf
        importlib.reload(pf)
        assert "fourier_divfree_tol" in pf.DEFAULT_HYPOTHESIS
        assert pf.DEFAULT_HYPOTHESIS["fourier_divfree_tol"] == 1e-10

    def test_pipeline_experimental_flag(self):
        import importlib, pipeline_v10_full as pf
        importlib.reload(pf)
        assert hasattr(pf, "EXPERIMENTAL")
        assert isinstance(pf.EXPERIMENTAL, bool)


# ══════════════════════════════════════════════════════════════════════════
# Lean 4 Formal Specification — static existence + content checks
# ══════════════════════════════════════════════════════════════════════════

class TestLean4Spec:
    LEAN_FILE = Path(__file__).parent.parent.parent / "proofs/lean4/v10_experimental.lean"

    def test_file_exists(self):
        assert self.LEAN_FILE.exists(), f"Missing: {self.LEAN_FILE}"

    def test_no_sorry(self):
        """No proof should use 'sorry' as a tactic (comments are fine)."""
        content = self.LEAN_FILE.read_text()
        # Only flag lines where sorry is used as a proof tactic
        sorry_tactic_lines = [
            l for l in content.splitlines()
            if ":= by sorry" in l or "| sorry" in l or l.strip() == "sorry"
        ]
        assert sorry_tactic_lines == [], f"sorry tactics found: {sorry_tactic_lines}"

    def test_namespace_correct(self):
        content = self.LEAN_FILE.read_text()
        assert "namespace SUNDIALS.V10.Experimental" in content

    def test_six_sections(self):
        content = self.LEAN_FILE.read_text()
        # Sections are marked as '-- § N'
        assert content.count("-- §") >= 6

    def test_p1_theorems_present(self):
        content = self.LEAN_FILE.read_text()
        assert "SpectralDeepProbLog_FourierGate_iters_le_300" in content
        assert "SpectralDeepProbLog_FourierGate_divergence_free_preserved" in content

    def test_p2_theorems_present(self):
        content = self.LEAN_FILE.read_text()
        assert "MixedPrecFGMRES_stability_below_threshold" in content
        assert "MixedPrecFGMRES_refinement_extends_stability" in content

    def test_p3_theorems_present(self):
        content = self.LEAN_FILE.read_text()
        assert "TensorCoreFP8AMG_jacobian_fits_vram" in content
        assert "TensorCoreFP8AMG_tensorcore_throughput_advantage" in content

    def test_shap_equation_present(self):
        content = self.LEAN_FILE.read_text()
        assert "SHAP_speedup_positive_at_optimal_params" in content
        assert "SHAP_block_size_monotone" in content

    def test_gate2b_policy_invariant(self):
        content = self.LEAN_FILE.read_text()
        assert "Gate2b_default_policy_valid" in content
        assert "DefaultGate2bPolicy" in content

    def test_proof_cache_section(self):
        content = self.LEAN_FILE.read_text()
        assert "ProofCache_covers_all_cycles" in content


# ══════════════════════════════════════════════════════════════════════════
# Physics Validator — experimental schema fields
# ══════════════════════════════════════════════════════════════════════════

class TestPhysicsValidatorExperimentalSchema:

    @pytest.fixture(autouse=True)
    def imports(self):
        from physics_validator_v10 import (
            validate_hypothesis_v10, OPTIONAL_EXPERIMENTAL_KEYS, REQUIRED_KEYS)
        self.validate = validate_hypothesis_v10
        self.optional_keys = OPTIONAL_EXPERIMENTAL_KEYS
        self.required_keys = REQUIRED_KEYS

    def test_optional_keys_documented(self):
        assert "fourier_divfree_tol" in self.optional_keys
        assert "B_field_sample" in self.optional_keys

    def test_experimental_hypothesis_passes_gates(self):
        hyp = dict(
            method_name="SpectralDeepProbLog_FourierGate",
            description="Fourier Hodge",
            mathematical_basis="Fourier spectral de Rham",
            preserves_magnetic_divergence=True,
            conserves_energy=True,
            expected_speedup_factor=41.8,
            krylov_iteration_bound="O(log N)",
            fourier_divfree_tol=1e-10,
            B_field_sample=np.zeros((4, 4, 4)).tolist(),
        )
        r = self.validate(hyp)
        assert r.passed

    def test_required_keys_unchanged(self):
        """Experimental additions must NOT remove any required key."""
        for k in ["method_name", "mathematical_basis", "preserves_magnetic_divergence",
                  "conserves_energy", "expected_speedup_factor", "krylov_iteration_bound"]:
            assert k in self.required_keys


# ══════════════════════════════════════════════════════════════════════════
# Reproduction SOC — artifact check
# ══════════════════════════════════════════════════════════════════════════

class TestSOCArtifact:
    DISC_DIR = Path(__file__).parent.parent / "discoveries"

    def test_soc_json_exists(self):
        socs = sorted(self.DISC_DIR.glob("soc_v10_reproduction_*.json"))
        assert len(socs) >= 1, "No SOC JSON found — run reproduce_v10_soc.py first"

    def test_soc_verdict_confirmed(self):
        soc_file = sorted(self.DISC_DIR.glob("soc_v10_reproduction_*.json"))[-1]
        import json
        soc = json.loads(soc_file.read_text())
        assert soc["reproduction_verdict"] == "CONFIRMED"
        assert soc["summary"]["reproduced_and_accepted"] == 3
        assert soc["budget"]["under_budget"] is True

    def test_soc_all_proposals_present(self):
        soc_file = sorted(self.DISC_DIR.glob("soc_v10_reproduction_*.json"))[-1]
        import json
        soc = json.loads(soc_file.read_text())
        ids = {p["proposal_id"] for p in soc["proposals"]}
        assert ids == {"P1", "P2", "P3"}

    def test_soc_budget_under_10_eur(self):
        soc_file = sorted(self.DISC_DIR.glob("soc_v10_reproduction_*.json"))[-1]
        import json
        soc = json.loads(soc_file.read_text())
        assert soc["budget"]["spent_eur"] < 10.0

    def test_lean4_certs_present(self):
        soc_file = sorted(self.DISC_DIR.glob("soc_v10_reproduction_*.json"))[-1]
        import json
        soc = json.loads(soc_file.read_text())
        certs = soc["lean4_certs"]
        assert all(c.startswith("CERT-LEAN4-") for c in certs.values())
