category: Testing & coverage
---

# Update production digest pins when canonical boundary registries change

Regenerating reader projections and updating fixture digests is not enough when
the canonical boundary changes. Production hashers can keep an old independent
digest constant and reject the newly generated registry. Update and test every
production digest pin alongside generated projections, case data, and reader
expectations; keep an executable negative control that proves the digest change
is detected.
