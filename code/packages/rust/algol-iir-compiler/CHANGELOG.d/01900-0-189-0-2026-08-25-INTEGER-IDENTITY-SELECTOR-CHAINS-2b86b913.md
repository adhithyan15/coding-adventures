## 0.189.0 — 2026-08-25 — integer identity selector chains

Bounded static while analysis now recognizes left-associative integer identity
chains composed from additive zero or multiplicative one. Mixed-precedence
chains remain structural, unsupported terms fail closed, and the ten-level
selector-dependency cap is unchanged.

