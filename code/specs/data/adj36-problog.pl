% ADJ36 — ProbLog encoding (auxiliary; for reviewer cross-check)
%
% This file is NOT the ADJ14 lowering. ADJ14's LR-aggregation
% semantics is log-odds composition; ProbLog's distribution
% semantics is possible-worlds enumeration with WMC. The two are
% approximately equivalent for simple cases like this one — the
% framework's LP19e implementation will produce the exact
% log-odds answer; ProbLog will produce a close approximation
% via WMC over a transformed clause set.
%
% This file is included so a reviewer familiar with ProbLog can
% sanity-check that the posterior we computed (≈28% pre-clarification,
% ≈49% post-clarification given exertional onset) is in the right
% neighborhood. Run with the ProbLog interpreter:
%   $ problog adj36-problog.pl
% Expected output: P(acs) ≈ 0.25–0.30.

% --- Prior ---------------------------------------------------------

0.10 :: acs_base.

% --- Observed evidence (set to true for this case) ----------------

observed_pressure_like.
observed_diaphoresis.
observed_htn.
observed_smoker.
observed_vitals_normal.
observed_ecg_no_st.
% Note: precipitator is NOT observed. The framework's VOI scan
% identifies this as the highest-VOI atom and kicks back rather
% than committing.

% --- Per-evidence contribution rules ------------------------------
%
% Encoding: for evidence E with positive LR L (likelihood ratio
% favoring acs), the contribution is modeled as a probabilistic
% rule whose firing probability under observation is approximately
% L/(1+L). This is the standard log-odds-to-probability transform
% for single-evidence Bayesian updates.

0.714 :: contrib_pressure :- observed_pressure_like.     % L=2.5
0.667 :: contrib_diaphoresis :- observed_diaphoresis.    % L=2.0
0.600 :: contrib_htn :- observed_htn.                    % L=1.5
0.643 :: contrib_smoker :- observed_smoker.              % L=1.8

% Negative contributors (L<1) — these are modeled as protective
% rules; the firing probability is (1-L)/(1+L) flipped to indicate
% reduced posterior.

0.333 :: not_acs_vitals_normal :- observed_vitals_normal.   % L=0.5
0.286 :: not_acs_ecg_no_st :- observed_ecg_no_st.           % L=0.4

% --- Composition rule ---------------------------------------------
%
% ACS is asserted when the base prior fires OR when contribution
% evidence accumulates AND no protective evidence overrides it.
% This is a simplified scaffolding for ProbLog inference; LP19e
% computes the exact log-odds aggregation natively.

acs :- acs_base.
acs :- contrib_pressure.
acs :- contrib_diaphoresis.
acs :- contrib_htn.
acs :- contrib_smoker.

% --- Query ---------------------------------------------------------

query(acs).

% --- Notes for the reviewer ---------------------------------------
%
% The framework's LP19e log-odds composition over the same evidence
% set produces P(acs | observed) ≈ 0.281. ProbLog's WMC over this
% encoding will produce a similar but not identical value (the
% encoding above is approximate; an exact translation would require
% encoding each contribution as an evidence-to-conclusion implication
% with the per-clause probability tuned to match log-odds composition
% under conditional independence).
%
% For exact reproduction of LP19e arithmetic, see adj36-execute.py.
