---
category: Rust
---

# A kernel must not type literals by trusting a constant's name; an environment can define Nat as False

lean-kernel slice 2 typed `Lit(Nat)` as the constant `Nat` after checking only that
some declaration *named* `Nat` existed. An environment can say `def Nat : Prop := False`;
then `theorem bad : False := 5` type-checked, because `5 : Nat` unfolds to `False`.
The name is chosen by whoever builds the environment, so it proves nothing about the
constant's meaning.

Fix: literals are `KernelError::Unsupported` until the primitive definitions
(`Nat` the inductive, `String` the structure) are validated (VER11a section 3.3).
Regression test: `literals_are_not_typed_by_a_constants_name` in
`tests/checker_hardening.rs`.

Rule: in a trusted checker, never attach meaning to a constant by its name alone. Anything
that special-cases a name (literals, `Eq`, `Quot`, `Nat.add` acceleration) must first
validate the declaration behind it, and fail closed until that validation exists. Write
the "environment redefines the name as False" probe before the feature.
