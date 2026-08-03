# Changelog

## 0.1.0

- Load configured Ed25519 package-verification keys from exact 32-byte raw files.
- Reject missing, changing, final-component-symlinked, and non-regular key paths.
- Reject non-canonical, identity, low-order, and mixed-order public keys.
- Map production and developer declarations to their Tier 3 and Tier 1 ceilings.
- Keep filesystem and key details out of stable operator-facing failures.
