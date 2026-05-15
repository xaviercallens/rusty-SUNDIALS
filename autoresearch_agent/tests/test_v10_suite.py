"""
rusty-SUNDIALS v10 — Full Validation Test Suite
Run: pytest tests/test_v10_suite.py -v -p no:xvfb
"""
import json, sys, math
import numpy as np
import pytest

sys.path.insert(0, "..")

# ══════════════════════════════════════════════════════════════════════════════
# Component 1 — Physics Validator (5-gate)
# ══════════════════════════════════════════════════════════════════════════════

from physics_validator_v10 import validate_hypothesis_v10

VALID_HYP = {
    "method_name": "FLAGNO_Test",
    "description": "Hodge-projected FNO for xMHD",
    "mathematical_basis": "Discrete de Rham",
    "preserves_magnetic_divergence": True,
    "conserves_energy": True,
    "expected_speedup_factor": 78.3,
    "krylov_iteration_bound": "O(1)",
}

class TestPhysicsValidator:
    def test_valid_hypothesis_passes_all_gates(self):
        r = validate_hypothesis_v10(json.dumps(VALID_HYP))
        assert r.passed, f"Expected pass, got: {r.first_failure}"
        assert len(r.gates) == 5

    def test_missing_schema_key_fails_gate1(self):
        bad = {k: v for k, v in VALID_HYP.items() if k != "method_name"}
        r = validate_hypothesis_v10(json.dumps(bad))
        assert not r.passed
        assert r.gates[0].gate == 1

    def test_div_b_violation_fails_gate3(self):
        bad = {**VALID_HYP, "preserves_magnetic_divergence": False}
        r = validate_hypothesis_v10(json.dumps(bad))
        assert not r.passed
        assert "∇·B" in (r.first_failure or "")

    def test_negative_speedup_fails(self):
        bad = {**VALID_HYP, "expected_speedup_factor": -1.0}
        r = validate_hypothesis_v10(json.dumps(bad))
        assert not r.passed

    def test_heuristic_keyword_rejection(self):
        bad = {**VALID_HYP, "method_name": "Unstable_Blowup_Solver"}
        r = validate_hypothesis_v10(json.dumps(bad))
        assert not r.passed

    def test_dict_input_accepted(self):
        r = validate_hypothesis_v10(VALID_HYP)
        assert r.passed

    def test_report_has_gates_list(self):
        r = validate_hypothesis_v10(VALID_HYP)
        assert len(r.gates) == 5
        assert all(hasattr(g, "passed") for g in r.gates)


# ══════════════════════════════════════════════════════════════════════════════
# Component 2 — SLURM / Vertex AI Job Manager
# ══════════════════════════════════════════════════════════════════════════════

from slurm_v10 import SlurmJobManager, JobType, JobStatus, BudgetGuard

class TestSlurmJobManager:
    def setup_method(self):
        self.mgr = SlurmJobManager(budget=100.0, use_real_gpu=False)

    def test_sbatch_creates_job(self):
        job = self.mgr.sbatch(JobType.SUNDIALS_SIM, {"n_dof": 128})
        assert job.job_id in self.mgr.jobs

    def test_wait_and_collect_succeeds(self):
        job = self.mgr.sbatch(JobType.SUNDIALS_SIM, {"n_dof": 64})
        result = self.mgr.wait_and_collect(job)
        assert job.status == JobStatus.SUCCEEDED
        assert "job_type" in result

    def test_cusparse_bench_job(self):
        job = self.mgr.sbatch(JobType.CUSPARSE_BENCH, {"dof_sizes": [64, 128]})
        result = self.mgr.wait_and_collect(job)
        assert "max_memory_reduction" in result
        assert result["max_memory_reduction"] > 1.0

    def test_federated_round_job(self):
        job = self.mgr.sbatch(JobType.FEDERATED_ROUND, {"n_clients": 2, "round_num": 1})
        result = self.mgr.wait_and_collect(job)
        assert result["n_clients"] == 2

    def test_rl_episode_job(self):
        job = self.mgr.sbatch(JobType.RL_EPISODE, {"episode": 0})
        result = self.mgr.wait_and_collect(job)
        assert "total_reward" in result

    def test_explainability_job(self):
        job = self.mgr.sbatch(JobType.EXPLAINABILITY, {})
        result = self.mgr.wait_and_collect(job)
        assert "shap_values" in result
        assert "discovered_equation" in result

    def test_budget_guard_raises_over_limit(self):
        guard = BudgetGuard(limit_usd=0.001)
        with pytest.raises(RuntimeError, match="Budget guard"):
            guard.check(1.0)

    def test_session_summary_keys(self):
        self.mgr.sbatch(JobType.SUNDIALS_SIM, {"n_dof": 64})
        summary = self.mgr.session_summary()
        assert "total_cost_usd" in summary
        assert "budget_remaining_usd" in summary

    def test_sacct_returns_dict(self):
        job = self.mgr.sbatch(JobType.SUNDIALS_SIM, {"n_dof": 64})
        acct = self.mgr.sacct(job.job_id)
        assert acct["job_id"] == job.job_id

    def test_squeue_returns_status(self):
        job = self.mgr.sbatch(JobType.SUNDIALS_SIM, {"n_dof": 64})
        status = self.mgr.squeue(job.job_id)
        assert status in list(JobStatus)


# ══════════════════════════════════════════════════════════════════════════════
# Component 3 — Peer Review (multi-LLM)
# ══════════════════════════════════════════════════════════════════════════════

from peer_review_v10 import run_peer_review, PeerReviewResult

REVIEW_HYP = {**VALID_HYP, "mathematical_basis": "Hodge decomposition"}
SIM_RESULTS = {"convergence_achieved": True, "fgmres_iterations": 3,
               "energy_drift": 1.2e-8, "divergence_error_max": 3.1e-14}
LEAN_CERT = "CERT-LEAN4-A3F2D1C09B4E"

class TestPeerReview:
    def test_fallback_returns_three_verdicts(self):
        result = run_peer_review(REVIEW_HYP, SIM_RESULTS,
                                 lean_cert=LEAN_CERT,
                                 reviewers=["gwen", "deepthink", "mistral"])
        assert len(result.verdicts) == 3

    def test_consensus_score_in_range(self):
        result = run_peer_review(REVIEW_HYP, SIM_RESULTS)
        assert 0.0 <= result.consensus_score <= 1.0

    def test_consensus_passed_is_bool(self):
        result = run_peer_review(REVIEW_HYP, SIM_RESULTS)
        assert isinstance(result.consensus_passed, bool)

    def test_each_verdict_has_required_fields(self):
        result = run_peer_review(REVIEW_HYP, SIM_RESULTS)
        for v in result.verdicts:
            assert hasattr(v, "reviewer")
            assert hasattr(v, "score")
            assert hasattr(v, "passed")

    def test_lean4_cert_stored_on_result(self):
        result = run_peer_review(REVIEW_HYP, SIM_RESULTS, lean_cert=LEAN_CERT)
        assert result.lean4_cert == LEAN_CERT

    def test_reviewer_subset_gwen_only(self):
        result = run_peer_review(REVIEW_HYP, SIM_RESULTS, reviewers=["gwen"])
        assert len(result.verdicts) == 1
        assert result.verdicts[0].reviewer == "gwen"

    def test_result_is_peer_review_result(self):
        result = run_peer_review(REVIEW_HYP, SIM_RESULTS)
        assert isinstance(result, PeerReviewResult)

    def test_to_dict_serializable(self):
        result = run_peer_review(REVIEW_HYP, SIM_RESULTS)
        d = result.to_dict()
        assert json.dumps(d, default=str)


# ══════════════════════════════════════════════════════════════════════════════
# Component 4 — cuSPARSE FP8 + AMGX
# ══════════════════════════════════════════════════════════════════════════════

from cusparse_amgx_v10 import (
    BlockSparseSensitivity, AMGXSolver, run_cusparse_amgx_benchmark
)
from scipy import sparse

class TestCuSparseAMGX:
    def _make_jacobian(self, n=64):
        d = np.ones(n) * 1e6
        o = -np.ones(n - 1) * 1e5
        return sparse.diags([d, o, o], [0, -1, 1], format="csr")

    def test_fp8_allocation_exact_nnz(self):
        J = self._make_jacobian(64)
        bss = BlockSparseSensitivity(n_dof=64, block_size=8)
        bss.fill_from_jacobian(J)
        assert bss._data_int8 is not None
        assert bss._nnz_blocks > 0

    def test_fp8_memory_less_than_fp64(self):
        J = self._make_jacobian(128)
        bss = BlockSparseSensitivity(n_dof=128, block_size=8)
        bss.fill_from_jacobian(J)
        bench = bss.benchmark()
        assert bench["fp8_memory_mb"] < bench["fp64_dense_memory_mb"]

    def test_issue_42_resolved_flag(self):
        J = self._make_jacobian(128)
        bss = BlockSparseSensitivity(n_dof=128, block_size=8)
        bss.fill_from_jacobian(J)
        assert bss.benchmark()["issue_42_resolved"] is True

    def test_amgx_solver_converges(self):
        n = 64
        J = self._make_jacobian(n)
        b = np.ones(n)
        solver = AMGXSolver()
        solver.fill_matrix(J)
        x, info = solver.solve(b)
        assert x.shape == (n,)
        assert info["iterations"] > 0

    def test_amgx_residual_small(self):
        n = 64
        J = self._make_jacobian(n)
        b = np.random.default_rng(0).standard_normal(n)
        solver = AMGXSolver()
        solver.fill_matrix(J)
        x, info = solver.solve(b)
        residual = np.linalg.norm(J @ x - b) / np.linalg.norm(b)
        assert residual < 0.1

    def test_benchmark_function_returns_result(self):
        result = run_cusparse_amgx_benchmark(n_dof=64, stiffness_ratio=1e4)
        assert result.memory_reduction >= 1.0
        assert result.amgx_iterations > 0
        assert result.backend in ("GPU-cupy", "CPU-pyamg")

    def test_memory_reduction_scales_with_dof(self):
        r1 = run_cusparse_amgx_benchmark(n_dof=64)
        r2 = run_cusparse_amgx_benchmark(n_dof=256)
        # Larger matrices should have better reduction
        assert r2.memory_reduction >= r1.memory_reduction * 0.5


# ══════════════════════════════════════════════════════════════════════════════
# Component 5 — Lean 4 Proof Cache
# ══════════════════════════════════════════════════════════════════════════════

from lean_proof_cache import try_auto_tactics, get_cached_proof, store_proof, proof_cache_stats

class TestLeanProofCache:
    def test_decide_tactic_on_simple_bound(self):
        result = try_auto_tactics("theorem t : 5 ≤ 7")
        assert result is not None
        assert result["auto_closed"] is True
        assert result["tactic_used"] == "decide"

    def test_equality_uses_simp_ring(self):
        result = try_auto_tactics("theorem t : (2 : ℕ) + 3 = 5")
        assert result is not None
        assert result["auto_closed"] is True
        assert result["tactic_used"] == "simp; ring"

    def test_store_and_retrieve_roundtrip(self):
        stmt = "theorem test_rt : 42 ≤ 100"
        # store: (theorem_stmt, proof_term, tactic_used, method_name)
        key = store_proof(stmt, "by decide", "decide", "TEST")
        assert key is not None
        cached = get_cached_proof(stmt)
        assert cached is not None
        assert cached["proof_term"] == "by decide"
        assert cached["tactic_used"] == "decide"

    def test_cache_stats_keys(self):
        stats = proof_cache_stats()
        assert "redis_connected" in stats
        assert "auto_tactics_available" in stats
        assert "memory_cached_proofs" in stats

    def test_sorry_theorem_returns_none(self):
        result = try_auto_tactics("theorem complex_plasma : sorry")
        assert result is None

    def test_cache_hit_after_auto_tactic(self):
        stmt = "theorem flagno_iters : 6 ≤ 7"
        try_auto_tactics(stmt, "FLAGNO")
        cached = get_cached_proof(stmt)
        assert cached is not None
        # Cache entry stores proof_term and tactic_used, not auto_closed
        assert "proof_term" in cached
        assert cached["proof_term"].startswith("by ")


# ══════════════════════════════════════════════════════════════════════════════
# Component 6 — Flower Federated Learning
# ══════════════════════════════════════════════════════════════════════════════

from federated_v10 import (
    run_federated_experiment, PrivacyGuard, FederatedServer,
    SundialsResearchClient, _fedavg
)

class TestFederatedLearning:
    def test_full_experiment_returns_final_model(self):
        result = run_federated_experiment(VALID_HYP, n_clients=2, n_rounds=2, n_local_experiments=1)
        assert "final_model" in result
        fm = result["final_model"]
        assert "final_speedup" in fm
        assert "rounds_completed" in fm

    def test_rounds_completed_matches_n_rounds(self):
        result = run_federated_experiment(VALID_HYP, n_clients=2, n_rounds=3, n_local_experiments=1)
        assert result["final_model"]["rounds_completed"] == 3

    def test_privacy_guard_blocks_raw_data(self):
        dirty = {"speedup": 10, "raw_b_field": [1, 2, 3], "coil_geometry": {}}
        clean = PrivacyGuard.sanitize(dirty)
        assert "raw_b_field" not in clean
        assert "coil_geometry" not in clean
        assert "speedup" in clean

    def test_privacy_audit_passes_clean(self):
        assert PrivacyGuard.audit({"speedup": 10, "converged": True}) is True

    def test_privacy_audit_fails_dirty(self):
        assert PrivacyGuard.audit({"raw_b_field": []}) is False

    def test_fedavg_weighted_mean(self):
        g1 = np.array([1.0, 2.0, 3.0])
        g2 = np.array([3.0, 4.0, 5.0])
        agg = _fedavg([g1, g2], weights=[1, 1])
        np.testing.assert_allclose(agg, [2.0, 3.0, 4.0])

    def test_fedavg_unequal_weights(self):
        g1 = np.array([0.0])
        g2 = np.array([4.0])
        agg = _fedavg([g1, g2], weights=[3, 1])
        np.testing.assert_allclose(agg, [1.0])

    def test_client_fit_returns_gradient(self):
        client = SundialsResearchClient(0, VALID_HYP, n_local_experiments=1)
        grad, n, metrics = client.fit(np.zeros(16), {"round": 1})
        assert grad.shape == (16,)
        assert n > 0
        assert "mean_speedup" in metrics

    def test_loss_history_has_correct_length(self):
        result = run_federated_experiment(VALID_HYP, n_clients=2, n_rounds=4, n_local_experiments=1)
        assert len(result["round_losses"]) == 4


# ══════════════════════════════════════════════════════════════════════════════
# Component 7 — PPO RL Agent
# ══════════════════════════════════════════════════════════════════════════════

from rl_agent_v10 import (
    SundialsEnv, decode_action, check_physics_constraints,
    MinimalPPO, train_ppo_agent, ACTION_DIM, OBS_DIM
)

class TestRLAgent:
    def test_decode_action_bounds(self):
        a = np.array([0.5] * ACTION_DIM)
        params = decode_action(a)
        assert 0 < params["coil_current_ma"] < 15
        assert 64 <= params["n_dof"] <= 1024
        assert params["block_size"] in [4, 8, 16, 32]

    def test_physics_constraint_valid_action(self):
        a = np.array([0.5, 0.5, 0.8, 0.5, 0.5, 0.5])
        valid, reason = check_physics_constraints(a)
        assert valid, reason

    def test_physics_constraint_blocks_zero_current(self):
        a = np.array([0.0, 0.5, 0.8, 0.5, 0.5, 0.5])
        valid, _ = check_physics_constraints(a)
        assert not valid

    def test_env_reset_returns_obs(self):
        env = SundialsEnv(max_steps=10)
        obs = env.reset(seed=42)
        assert obs.shape == (OBS_DIM,)
        assert np.all(obs >= 0)

    def test_env_step_physics_violation_gives_penalty(self):
        env = SundialsEnv(max_steps=10)
        env.reset(seed=0)
        bad_action = np.zeros(ACTION_DIM)  # coil=0 → invalid
        _, reward, _, info = env.step(bad_action)
        assert reward == -10.0
        assert info.get("invalid") is True

    def test_env_step_valid_action(self):
        env = SundialsEnv(max_steps=10)
        env.reset(seed=1)
        good = np.array([0.5, 0.3, 0.8, 0.5, 0.5, 0.5])
        obs, reward, done, info = env.step(good)
        assert obs.shape == (OBS_DIM,)
        assert isinstance(done, bool)

    def test_minimal_ppo_get_action_shape(self):
        policy = MinimalPPO(OBS_DIM, ACTION_DIM)
        obs = np.random.default_rng(0).uniform(0, 1, OBS_DIM)
        action, log_prob = policy.get_action(obs)
        assert action.shape == (ACTION_DIM,)
        assert np.all((action >= 0) & (action <= 1))

    def test_minimal_ppo_update(self):
        policy = MinimalPPO(OBS_DIM, ACTION_DIM)
        rollouts = [
            {"obs": np.random.rand(OBS_DIM), "action": np.random.rand(ACTION_DIM),
             "reward": float(i), "log_prob": -1.0}
            for i in range(5)
        ]
        info = policy.update(rollouts)
        assert "mean_return" in info

    def test_train_ppo_returns_result(self):
        result = train_ppo_agent(n_episodes=3, max_steps_per_episode=5)
        assert result.episodes == 3
        assert isinstance(result.best_reward, float)
        assert result.backend in ("stable-baselines3", "minimal-ppo-numpy")

    def test_reward_history_length(self):
        result = train_ppo_agent(n_episodes=4, max_steps_per_episode=5)
        assert len(result.reward_history) == 4


# ══════════════════════════════════════════════════════════════════════════════
# Component 8 — SHAP + PySR Explainability
# ══════════════════════════════════════════════════════════════════════════════

from explainability_v10 import (
    generate_synthetic_dataset, compute_shap, run_pysr,
    run_explainability_pipeline, FEATURE_NAMES, TARGET_NAMES
)

class TestExplainability:
    def setup_method(self):
        self.X, self.Y = generate_synthetic_dataset(n_samples=100, seed=0)

    def test_dataset_shape(self):
        assert self.X.shape == (100, len(FEATURE_NAMES))
        assert self.Y.shape == (100, len(TARGET_NAMES))

    def test_speedup_positive(self):
        assert np.all(self.Y[:, 0] > 0), "Speedup must always be positive"

    def test_shap_top_k_length(self):
        result = compute_shap(self.X, self.Y, FEATURE_NAMES, target_idx=0, top_k=3)
        assert len(result.top_k_features) == 3

    def test_shap_mean_abs_sums_to_positive(self):
        result = compute_shap(self.X, self.Y, FEATURE_NAMES, target_idx=0)
        assert result.mean_abs_shap.sum() > 0

    def test_shap_features_are_valid_names(self):
        result = compute_shap(self.X, self.Y, FEATURE_NAMES, target_idx=0, top_k=4)
        for f in result.top_k_features:
            assert f in FEATURE_NAMES

    def test_pysr_fallback_r2_reasonable(self):
        shap_r = compute_shap(self.X, self.Y, FEATURE_NAMES, target_idx=0, top_k=3)
        pysr_r = run_pysr(self.X, self.Y[:, 0], FEATURE_NAMES,
                          shap_r.top_k_indices, target_name="speedup")
        assert -1.0 <= pysr_r.r2_score <= 1.0
        assert pysr_r.backend in ("pysr", "polynomial-fallback")

    def test_pysr_equation_string_nonempty(self):
        shap_r = compute_shap(self.X, self.Y, FEATURE_NAMES, target_idx=0, top_k=3)
        pysr_r = run_pysr(self.X, self.Y[:, 0], FEATURE_NAMES,
                          shap_r.top_k_indices, target_name="speedup")
        assert len(pysr_r.discovered_equation) > 3

    def test_full_pipeline_runs(self):
        report = run_explainability_pipeline(n_samples=80, top_k_features=3, targets=[0])
        assert len(report.shap_results) == 1
        assert len(report.pysr_results) == 1
        assert len(report.top_global_features) > 0

    def test_latex_table_generated(self):
        report = run_explainability_pipeline(n_samples=80, top_k_features=3, targets=[0])
        latex = report.to_latex_table()
        assert r"\begin{table}" in latex
        assert r"$R^2$" in latex

    def test_n_dof_in_top_features(self):
        report = run_explainability_pipeline(n_samples=150, top_k_features=4, targets=[0])
        # n_dof is the dominant driver of speedup by construction
        assert "n_dof" in report.top_global_features


# ══════════════════════════════════════════════════════════════════════════════
# Neuro-Symbolic Gate (integrated)
# ══════════════════════════════════════════════════════════════════════════════

from neuro_symbolic_v10 import validate_neuro_symbolic, evaluate_physics

class TestNeuroSymbolic:
    def test_valid_passes_all_gates(self):
        report = validate_neuro_symbolic(VALID_HYP)
        assert report.passed

    def test_div_b_violation_caught(self):
        bad = {**VALID_HYP, "preserves_magnetic_divergence": False}
        report = validate_neuro_symbolic(bad)
        assert not report.passed
        assert "∇·B" in (report.first_failure or "")

    def test_energy_violation_caught(self):
        bad = {**VALID_HYP, "conserves_energy": False}
        report = validate_neuro_symbolic(bad)
        assert not report.passed

    def test_backwards_compat_shim(self):
        passed, reason = evaluate_physics(VALID_HYP)
        assert isinstance(passed, bool)
        assert isinstance(reason, str)

    def test_gates_list_populated(self):
        report = validate_neuro_symbolic(VALID_HYP)
        assert len(report.gates) >= 3

    def test_schema_gate_first(self):
        report = validate_neuro_symbolic(VALID_HYP)
        assert report.gates[0].gate == 1
        assert report.gates[0].name == "Schema"

    def test_bad_json_string(self):
        report = validate_neuro_symbolic("not valid json {{{")
        assert not report.passed
        assert "JSON parse error" in (report.first_failure or "")

    def test_speedup_out_of_bounds(self):
        bad = {**VALID_HYP, "expected_speedup_factor": 2e6}
        report = validate_neuro_symbolic(bad)
        assert not report.passed


# ══════════════════════════════════════════════════════════════════════════════
# End-to-end: pipeline_v10_full integration
# ══════════════════════════════════════════════════════════════════════════════

from pipeline_v10_full import run_full_pipeline

class TestFullPipeline:
    def test_pipeline_component4_succeeds(self):
        out = run_full_pipeline(use_real_gpu=False, components=[4], budget_usd=100)
        assert "component_4" in out["results"]
        assert "error" not in out["results"]["component_4"]

    def test_pipeline_component6_succeeds(self):
        out = run_full_pipeline(use_real_gpu=False, components=[6], budget_usd=100)
        assert "component_6" in out["results"]
        r = out["results"]["component_6"]
        assert "error" not in r
        assert r["final_model"]["rounds_completed"] > 0

    def test_pipeline_component8_succeeds(self):
        out = run_full_pipeline(use_real_gpu=False, components=[8], budget_usd=100)
        assert "component_8" in out["results"]
        assert "error" not in out["results"]["component_8"]

    def test_budget_remaining_not_negative(self):
        out = run_full_pipeline(use_real_gpu=False, components=[2], budget_usd=100)
        assert out["session"]["budget_remaining_usd"] >= 0

    def test_output_json_serializable(self):
        out = run_full_pipeline(use_real_gpu=False, components=[4], budget_usd=100)
        serialized = json.dumps(out, default=str)
        assert len(serialized) > 100
