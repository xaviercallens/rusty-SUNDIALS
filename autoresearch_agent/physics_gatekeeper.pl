% DeepProbLog: The Physics Gatekeeper
% This script prevents the LLM from writing mathematically unstable code
% by enforcing Extended Magnetohydrodynamics (xMHD) invariants.

% 1. Energy Conservation: Numerical method cannot generate spurious energy.
nnz(energy_bounded(Method)).
energy_bounded(Method) :-
    is_symplectic(Method); 
    (is_imex(Method), satisfies_l2_stability(Method)).

% 2. Divergence-Free Magnetic Field (∇ ⋅ B = 0)
nnz(div_b_zero(Method)).
div_b_zero(Method) :-
    uses_constrained_transport(Method).

% 3. Evaluate an AI-proposed paradigm
evaluate_paradigm(Paradigm, P) :-
    prob(energy_bounded(Paradigm), P1),
    prob(div_b_zero(Paradigm), P2),
    P is P1 * P2.

% Queries to test new methods
query(evaluate_paradigm(dynamic_spectral_imex)).
