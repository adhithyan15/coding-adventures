## 0.506.0 (UNRELEASED)

- Replaces the hand-enumerated `sign`-rooted additive offset budget (forty-four units after forty-four one-unit releases) with one rule: every variable-free additive term must be a whole number, and the checked total per runtime path may reach `2^53 - 1` units (`MAX_SIGN_OFFSET_UNITS`), so `sign(x) ± offset` stays an exact binary64 integer. Fractional terms, totals of `2^53` or more, and variable terms still fail closed. The previous cap protected no property the outer runtime-real selector rule does not already accept for finite static terms.
