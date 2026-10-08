---
category: Security boundaries
---

# Allocator-state compatibility must not become complete provenance through checked queries

The independent CV02 checkpoint review found that full allocation coverage and
enabled recording were validated only when a checked mutation ledger existed.
A generic compact reload could retain an intentionally unstored allocation,
then return successful try_lineage/ancestors/descendants for a later stored ID.

The regression reproduces that gap, including a disabled fully stored log.
Evidence validation now rejects disabled recording and compact allocation gaps
regardless of the constructor/import path. Generic snapshot save/reload keeps
its allocator-state contract, but fallible graph queries cannot upgrade that
compatibility state into a claim of complete provenance.

Validate evidence at the consuming boundary. A private checked-policy flag is
not proof that all other construction paths can safely bypass graph checks.
