"""
Tests for autoresearch_agent/hypothesis_validator_v11.py
"""
import pytest
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from hypothesis_validator_v11 import HypothesisValidator, ValidationResult


@pytest.fixture
def validator():
    return HypothesisValidator(confidence_threshold=0.75)


class TestSymPyValidator:
    def test_valid_power_law(self, validator):
        result = validator.validate("speedup = 78.3 * n_dof**0.5 / krylov_restart**0.1")
        assert result.symbolic_form is not None, "Expected LaTeX symbolic form"
        assert "$" in result.symbolic_form

    def test_invalid_syntax_returns_result(self, validator):
        result = validator.validate("invalid @@@ garbage")
        assert isinstance(result, ValidationResult)
        assert result.verdict in ("VALID", "UNCERTAIN", "INVALID")

    def test_energy_drift_marked_invalid_or_uncertain(self, validator):
        result = validator.validate("energy_drift = alpha * dt**2 + beta * stiffness")
        # Should fail SymPy parse (alpha/beta/stiffness undefined as functions)
        # or fall into uncertain/invalid due to dimensional issues
        assert result.confidence < 0.99  # not blindly accepting


class TestDeepProbLogValidator:
    def test_power_law_scores_above_threshold(self, validator):
        result = validator.validate("speedup = 78.3 * n_dof**0.5 / krylov_restart**0.1")
        assert result.problog_score > 0.70, (
            f"Power-law hypothesis should score > 0.70, got {result.problog_score}"
        )

    def test_neural_approximation_penalized(self, validator):
        result = validator.validate("speedup ~ neural_approx(x)")
        # Neural approximation gets a 0.75× penalty factor
        assert result.problog_score < 0.85


class TestHypothesisValidator:
    def test_high_confidence_is_valid(self, validator):
        # This hypothesis is syntactically clean and physically plausible
        result = validator.validate("speedup = 78.3 * n_dof**0.5 / krylov_restart**0.1")
        assert result.verdict == "VALID"
        assert result.confidence >= 0.75

    def test_invalid_hypothesis_has_counterexample(self):
        strict = HypothesisValidator(confidence_threshold=0.99)
        result = strict.validate("invalid @@@ garbage !!!")
        if result.verdict == "INVALID":
            assert result.counterexample is not None

    def test_verdict_str_present(self, validator):
        result = validator.validate("speedup = n_dof**0.5")
        assert result.verdict in ("VALID", "UNCERTAIN", "INVALID")

    def test_str_representation(self, validator):
        result = validator.validate("speedup = 10.0 * n_dof**0.5")
        text = str(result)
        assert "Verdict" in text
        assert "Hypothesis" in text

    def test_custom_threshold(self):
        # Low threshold — most hypotheses should pass
        lax = HypothesisValidator(confidence_threshold=0.10)
        result = lax.validate("speedup = n_dof**0.5")
        assert result.verdict == "VALID"

    def test_confidence_in_unit_interval(self, validator):
        for h in [
            "speedup = 78.3 * n_dof**0.5",
            "garbage",
            "convergence_rate = exp(-gamma * t)",
        ]:
            r = validator.validate(h)
            assert 0.0 <= r.confidence <= 1.0, f"Confidence out of bounds for: {h}"
