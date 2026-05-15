"""
Tests for autoresearch_agent/federated_v11.py
"""
import math
import pytest
import numpy as np
import sys, os

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from federated_v11 import (
    HpcSite, CEA_CADARACHE, ITER_ORG, TU_MUNICH, DEFAULT_SITES,
    NativeGaussianAccountant,
    _clip_gradient, _add_gaussian_noise, _dp_fedavg,
    _results_to_gradient, _gradient_to_metrics,
    DPFederatedClient, DPFederatedOrchestrator,
    run_federated_dp_experiment,
)

SIMPLE_HYP = {
    "method_name": "TestDP",
    "expected_speedup_factor": 10.0,
    "preserves_magnetic_divergence": True,
}


# ── Site definitions ──────────────────────────────────────────────────────────

class TestHpcSite:
    def test_default_sites_count(self):
        assert len(DEFAULT_SITES) == 3

    def test_site_names(self):
        names = {s.name for s in DEFAULT_SITES}
        assert "CEA Cadarache" in names
        assert "ITER Organization" in names
        assert "TU Munich HPC" in names

    def test_site_ids_unique(self):
        ids = [s.site_id for s in DEFAULT_SITES]
        assert len(ids) == len(set(ids))

    def test_iter_has_largest_grid(self):
        assert ITER_ORG.n_dof > CEA_CADARACHE.n_dof
        assert ITER_ORG.n_dof > TU_MUNICH.n_dof


# ── DP utilities ──────────────────────────────────────────────────────────────

class TestNativeGaussianAccountant:
    def test_initial_epsilon_infinite(self):
        acc = NativeGaussianAccountant(noise_multiplier=1.1, delta=1e-5)
        assert acc.get_epsilon() == float("inf")

    def test_epsilon_increases_with_steps(self):
        acc = NativeGaussianAccountant(noise_multiplier=1.1, delta=1e-5)
        acc.step()
        eps1 = acc.get_epsilon()
        acc.step()
        eps2 = acc.get_epsilon()
        assert eps2 > eps1

    def test_higher_noise_lower_epsilon(self):
        a1 = NativeGaussianAccountant(noise_multiplier=0.5,  delta=1e-5)
        a2 = NativeGaussianAccountant(noise_multiplier=2.0,  delta=1e-5)
        a1.step(); a2.step()
        assert a1.get_epsilon() > a2.get_epsilon()

    def test_epsilon_finite_after_steps(self):
        acc = NativeGaussianAccountant(noise_multiplier=1.1, delta=1e-5)
        for _ in range(5):
            acc.step()
        assert math.isfinite(acc.get_epsilon())


class TestClipping:
    def test_clip_within_norm_unchanged(self):
        g = np.array([0.3, 0.4])  # ‖g‖ = 0.5 < 1.0
        clipped = _clip_gradient(g, max_norm=1.0)
        np.testing.assert_allclose(clipped, g)

    def test_clip_above_norm_rescaled(self):
        g = np.array([3.0, 4.0])  # ‖g‖ = 5.0
        clipped = _clip_gradient(g, max_norm=1.0)
        assert np.linalg.norm(clipped) <= 1.0 + 1e-9

    def test_clip_direction_preserved(self):
        g = np.array([3.0, 4.0])
        clipped = _clip_gradient(g, max_norm=2.0)
        # Direction unchanged
        cos_sim = np.dot(g / np.linalg.norm(g), clipped / np.linalg.norm(clipped))
        assert cos_sim > 0.999


class TestGaussianNoise:
    def test_noise_shape_preserved(self):
        g = np.ones(32)
        noisy = _add_gaussian_noise(g, noise_multiplier=1.0, max_norm=1.0, seed=0)
        assert noisy.shape == g.shape

    def test_noise_changes_gradient(self):
        g = np.ones(32)
        noisy = _add_gaussian_noise(g, noise_multiplier=1.0, max_norm=1.0, seed=42)
        assert not np.allclose(noisy, g)

    def test_zero_noise_multiplier_adds_no_noise(self):
        g = np.ones(32)
        noisy = _add_gaussian_noise(g, noise_multiplier=0.0, max_norm=1.0, seed=0)
        np.testing.assert_allclose(noisy, g)


class TestDPFedAvg:
    def test_fedavg_output_shape(self):
        gs = [np.ones(16) * i for i in range(3)]
        ws = [1, 1, 1]
        agg = _dp_fedavg(gs, ws, noise_multiplier=0.0, max_norm=100.0, seed=0)
        assert agg.shape == (16,)

    def test_fedavg_weighted_mean(self):
        # With zero noise: agg should be weighted mean of clipped gradients
        g0 = np.ones(8) * 2.0
        g1 = np.ones(8) * 4.0
        agg = _dp_fedavg([g0, g1], [1, 1], noise_multiplier=0.0, max_norm=100.0, seed=0)
        np.testing.assert_allclose(agg, np.ones(8) * 3.0, atol=1e-9)

    def test_noise_makes_result_different(self):
        gs = [np.ones(16) for _ in range(3)]
        ws = [1, 1, 1]
        agg0 = _dp_fedavg(gs, ws, noise_multiplier=0.0, max_norm=1.0, seed=0)
        agg1 = _dp_fedavg(gs, ws, noise_multiplier=2.0, max_norm=1.0, seed=0)
        assert not np.allclose(agg0, agg1)


# ── DP Client ────────────────────────────────────────────────────────────────

class TestDPFederatedClient:
    def test_compute_gradient_returns_correct_shape(self):
        client = DPFederatedClient(CEA_CADARACHE, SIMPLE_HYP, n_local=2)
        g, n = client.compute_gradient(round_num=1)
        assert g.shape == (32,)
        assert n == 2

    def test_clipped_norm_within_bound(self):
        client = DPFederatedClient(CEA_CADARACHE, SIMPLE_HYP, max_norm=1.0, n_local=2)
        g, _ = client.compute_gradient(round_num=1)
        assert np.linalg.norm(g) <= 1.0 + 1e-6


# ── Orchestrator ─────────────────────────────────────────────────────────────

class TestDPFederatedOrchestrator:
    def test_runs_all_rounds(self):
        orch = DPFederatedOrchestrator(
            sites=DEFAULT_SITES, n_rounds=3,
            noise_multiplier=0.5, epsilon_budget=100.0
        )
        rounds = orch.run(SIMPLE_HYP)
        assert len(rounds) == 3

    def test_epsilon_increases_per_round(self):
        orch = DPFederatedOrchestrator(
            sites=DEFAULT_SITES, n_rounds=4,
            noise_multiplier=1.1, epsilon_budget=100.0
        )
        rounds = orch.run(SIMPLE_HYP)
        epsilons = [r.epsilon_spent for r in rounds if math.isfinite(r.epsilon_spent)]
        if len(epsilons) >= 2:
            assert epsilons[-1] >= epsilons[0]

    def test_stops_at_epsilon_budget(self):
        orch = DPFederatedOrchestrator(
            sites=DEFAULT_SITES, n_rounds=100,
            noise_multiplier=0.01,  # very low noise → high ε
            epsilon_budget=0.5,
        )
        rounds = orch.run(SIMPLE_HYP)
        # Should have stopped well before 100 rounds
        assert len(rounds) < 100

    def test_final_model_keys(self):
        orch = DPFederatedOrchestrator(
            sites=DEFAULT_SITES, n_rounds=2,
            noise_multiplier=1.1, epsilon_budget=100.0
        )
        orch.run(SIMPLE_HYP)
        model = orch.final_model()
        assert "final_speedup" in model
        assert "privacy_budget" in model
        assert "rounds_completed" in model

    def test_privacy_budget_structure(self):
        orch = DPFederatedOrchestrator(
            sites=DEFAULT_SITES, n_rounds=2,
            noise_multiplier=1.1, delta=1e-5, epsilon_budget=100.0
        )
        orch.run(SIMPLE_HYP)
        pb = orch.final_model()["privacy_budget"]
        assert "epsilon_spent" in pb
        assert "delta" in pb
        assert pb["delta"] == pytest.approx(1e-5)
        assert math.isfinite(pb["epsilon_spent"])


# ── Public API ────────────────────────────────────────────────────────────────

class TestRunFederatedDP:
    def test_returns_privacy_budget(self):
        result = run_federated_dp_experiment(SIMPLE_HYP, n_rounds=2)
        assert "privacy_budget" in result
        assert "epsilon_spent" in result["privacy_budget"]

    def test_privacy_guarantee_string(self):
        result = run_federated_dp_experiment(SIMPLE_HYP, n_rounds=2)
        assert "DP" in result["privacy_guarantee"]
        assert "Raw plasma" in result["privacy_guarantee"]

    def test_round_epsilons_list(self):
        result = run_federated_dp_experiment(SIMPLE_HYP, n_rounds=3)
        assert len(result["round_epsilons"]) == 3

    def test_three_sites_used(self):
        result = run_federated_dp_experiment(SIMPLE_HYP, n_rounds=2)
        assert result["n_sites"] == 3
