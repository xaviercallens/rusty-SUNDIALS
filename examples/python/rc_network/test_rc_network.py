"""pytest: the CVODE binding passes the RC-network K1/K2 controls.

    pip install numpy scipy pytest && pytest examples/python/rc_network
"""
import pytest

pytest.importorskip("rusty_sundials")

from rc_network_benchmark import CASES, run_case  # noqa: E402


@pytest.mark.parametrize("case", CASES)
def test_rc_network_controls(case):
    r = run_case(case)
    assert r["K1_pass"], f"K1 known-answer error {r['K1_rel_err']:.2e} >= 1e-5"
    assert r["K2_pass"], f"K2 steady-state vs DtN error {r['K2_rel_err']:.2e} >= 1e-6"
