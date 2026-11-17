# vos-sched-contract-check

Test-only companion of [`vos-sched-contract`](../vos-sched-contract) (VOS-010).

It joins the contract's Warrant `.part` files into a document the way
`vos-link gen-doc` (VOS-033) will, then uses the repository's own Warrant tooling
(`coding_adventures_warrant_grammar`: parser, `check_warrant`, ground `Evaluator`) to:

1. type-check the contract block and a generated-shape document (bound operations,
   `labels`/`input`/`bound` rows, every obligations block);
2. evaluate the nine model-level lemmas over their whole domains at `N = 8`;
3. evaluate every obligation over every binder value against three implementations
   written as Warrant models (a fixed-priority bitmap, a one-cell-per-thread scan,
   the most-general oracle scheduler);
4. a table of negative controls: 19 wrong implementations and 9 lemma mutants. Each is a
   textual change asserted to have changed the model; the claims whose text names a
   mutated operation are evaluated and the set that fails must equal the expected set
   exactly. A test asserts that every obligation, and every conjunct of the compound
   ones (`_out` and `_img` of insert and remove and dispatch, the `.choice == c`
   conjunct of `dispatch_sound`), is broken by some mutant. Limits (VOS10 section 12):
   the `_post` claims are implied by their `_img` twins, so they are asserted as a pair;
   the canonical block is blind to a consistently wrong `build`; `closure_at_m` is a
   trivial lemma;
5. check the Rust enums of the contract crate against the Warrant variants.

It makes no claim about compiled Rust: that is the evidence harness of VOS-011 and
VOS-012. It exists as its own crate because the shape check (VOS03, SH12) allows a
class K component crate no Warrant-tooling dependency; its `vos.toml` is class H.
It is folded into `vos-link`'s tests when VOS-033 lands (backlog VOS-D049).

```text
cargo test -p vos-sched-contract-check      # about a minute: several million claim evaluations
```
