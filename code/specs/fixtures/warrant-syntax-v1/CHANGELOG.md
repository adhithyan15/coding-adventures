# Changelog

## Unreleased

- Author 57 source/verdict/subtree syntax cases across nine reviewed feature
  groups, with accepted/rejected baselines and semantic-invalid parser positives.
- Add strict JSON-to-TFV projection, byte-level drift checks and a Rust L0
  consumer with exact witnesses, compiled-data agreement and negative controls.
- Leave typed/binding/solver cases and protected source admission to subsequent
  VER04 stages; this corpus is syntax evidence only.

## Unreleased (ADL-001a)

- Add four cases for the `profile algorithm` header (57 to 61): the accepted
  `warrant m v1 profile algorithm` with its `profile_clause` witness, and three
  rejections (no name, a second clause, the clause before the version word).
  The grammar cannot see which profile names are legal, so the version/profile
  *semantics* (W0101, W0801) are in `warrant-profile-algorithm-v1`.
