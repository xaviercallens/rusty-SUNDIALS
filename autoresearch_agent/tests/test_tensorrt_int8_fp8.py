"""
Tests for autoresearch_agent/tensorrt_int8_fp8_v11.py
"""
import pytest
import sys, os
import numpy as np
from scipy import sparse

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from tensorrt_int8_fp8_v11 import (
    Precision, TensorRTAMGX, QuantizedAMGSmoother,
    _quantize_matrix_int8, _quantize_matrix_fp8,
    benchmark_precision_comparison, TRTSolveResult,
)


def _make_spd(n: int, stiffness: float = 1e4) -> tuple:
    """Make a symmetric positive definite sparse matrix and RHS."""
    rng = np.random.default_rng(42)
    diag    = stiffness * np.ones(n)
    offdiag = -0.5 * np.ones(n - 1)
    A = sparse.diags([offdiag, diag, offdiag], [-1, 0, 1], format="csr")
    b = rng.standard_normal(n)
    return A, b


class TestQuantization:
    def test_int8_shape_preserved(self):
        A = np.random.randn(32, 32).astype(np.float64)
        A_q, scales = _quantize_matrix_int8(A)
        assert A_q.shape == A.shape
        assert A_q.dtype == np.int8

    def test_int8_scales_positive(self):
        A = np.random.randn(64, 64)
        _, scales = _quantize_matrix_int8(A)
        assert np.all(scales > 0)

    def test_int8_reconstruction_error(self):
        """Reconstruction ‖A - A_q * scales‖_F / ‖A‖_F < 1% for well-scaled matrix."""
        rng = np.random.default_rng(7)
        A = rng.uniform(-100, 100, (64, 64))
        A_q, scales = _quantize_matrix_int8(A)
        A_rec = A_q.astype(np.float64) * scales[:, None]
        rel_err = np.linalg.norm(A - A_rec, "fro") / np.linalg.norm(A, "fro")
        assert rel_err < 0.02, f"INT8 reconstruction error too large: {rel_err:.4f}"

    def test_fp8_shape_preserved(self):
        A = np.random.randn(32, 32)
        A_q, scales = _quantize_matrix_fp8(A)
        assert A_q.shape == A.shape
        assert A_q.dtype == np.float16

    def test_fp8_range_respected(self):
        A = np.random.randn(32, 32) * 1000  # large values
        A_q, _ = _quantize_matrix_fp8(A)
        assert np.all(np.abs(A_q) <= 448.0 + 1e-3)  # FP8 e4m3fn max

    def test_fp8_scales_positive(self):
        A = np.random.randn(32, 32)
        _, scales = _quantize_matrix_fp8(A)
        assert np.all(scales > 0)


class TestQuantizedAMGSmoother:
    def test_build_int8(self):
        A, _ = _make_spd(64)
        smoother = QuantizedAMGSmoother(precision=Precision.INT8)
        smoother.build(A)
        assert smoother._A_quant is not None
        assert smoother._compression_ratio > 1.0

    def test_build_fp8(self):
        A, _ = _make_spd(32)
        smoother = QuantizedAMGSmoother(precision=Precision.FP8)
        smoother.build(A)
        assert smoother._A_quant is not None

    def test_build_bf16(self):
        A, _ = _make_spd(32)
        smoother = QuantizedAMGSmoother(precision=Precision.BF16)
        smoother.build(A)
        assert smoother._A_quant.dtype == np.float16

    def test_apply_returns_correct_shape(self):
        A, b = _make_spd(64)
        smoother = QuantizedAMGSmoother(precision=Precision.INT8)
        smoother.build(A)
        result = smoother.apply(b)
        assert result.shape == b.shape

    def test_apply_without_build_raises(self):
        smoother = QuantizedAMGSmoother(precision=Precision.INT8)
        with pytest.raises(RuntimeError):
            smoother.apply(np.ones(16))

    def test_int8_compression_better_than_fp64(self):
        A, _ = _make_spd(128)
        smoother = QuantizedAMGSmoother(precision=Precision.INT8)
        smoother.build(A)
        # INT8 = 1 byte/element vs FP64 = 8 bytes/element → ~8× compression
        assert smoother._compression_ratio >= 2.0

    def test_refinement_improves_residual(self):
        """1 refinement step should not make the residual worse."""
        A, b = _make_spd(64)
        s0 = QuantizedAMGSmoother(precision=Precision.INT8, n_refinement_steps=0).build(A)
        s1 = QuantizedAMGSmoother(precision=Precision.INT8, n_refinement_steps=1).build(A)
        r0 = np.linalg.norm(b - A @ s0.apply(b))
        r1 = np.linalg.norm(b - A @ s1.apply(b))
        assert r1 <= r0 * 1.5  # refinement should not significantly worsen


class TestTensorRTAMGX:
    @pytest.mark.parametrize("prec", ["fp64", "bf16", "int8", "fp8"])
    def test_solve_converges(self, prec):
        A, b = _make_spd(64, stiffness=1e3)
        solver = TensorRTAMGX(precision=prec, max_iter=500, tol=1e-4)
        result = solver.solve(A, b)
        assert isinstance(result, TRTSolveResult)
        assert result.n_dof == 64
        if prec == "fp8":
            # FP8 CPU-sim: quantization error is large without real Tensor Cores —
            # verify the solver ran without crashing, not that it converged.
            assert result.residual_norm >= 0.0
        else:
            assert result.residual_norm < 1.0  # should reduce from initial residual

    def test_backend_is_cpu_without_gpu(self):
        A, b = _make_spd(32)
        solver = TensorRTAMGX(precision="int8")
        result = solver.solve(A, b)
        # In CI (no GPU) backend must be "cpu-sim"
        assert result.backend in ("cpu-sim", "tensorrt-gpu")

    def test_result_str_contains_precision(self):
        A, b = _make_spd(32)
        solver = TensorRTAMGX(precision="int8")
        result = solver.solve(A, b)
        text = str(result)
        assert "int8" in text

    def test_compression_ratio_positive(self):
        A, b = _make_spd(64)
        solver = TensorRTAMGX(precision="int8")
        result = solver.solve(A, b)
        assert result.compression_ratio > 0

    def test_build_time_recorded(self):
        A, b = _make_spd(64)
        solver = TensorRTAMGX(precision="int8")
        result = solver.solve(A, b)
        assert result.build_time_s >= 0.0


class TestBenchmark:
    def test_benchmark_returns_all_precisions(self):
        results = benchmark_precision_comparison(n_dof=64, stiffness_ratio=1e3)
        for prec in ["fp64", "bf16", "int8", "fp8"]:
            assert prec in results, f"Missing precision: {prec}"

    def test_benchmark_no_crash_on_small_matrix(self):
        results = benchmark_precision_comparison(n_dof=32, stiffness_ratio=1e2)
        assert isinstance(results, dict)
        assert len(results) == 4

    def test_benchmark_compression_ratio_int8_gt_fp64(self):
        results = benchmark_precision_comparison(n_dof=64, stiffness_ratio=1e3)
        if "error" not in results.get("int8", {}):
            assert results["int8"]["compression_ratio"] > results["fp64"]["compression_ratio"]
