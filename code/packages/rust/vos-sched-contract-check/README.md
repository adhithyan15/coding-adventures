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
4. check that each obligation can fail, with wrong implementations (a negative
   control per claim);
5. check the Rust enums of the contract crate against the Warrant variants.

It makes no claim about compiled Rust: that is the evidence harness of VOS-011 and
VOS-012. It exists as its own crate because the shape check (VOS03, SH12) allows a
class K component crate no Warrant-tooling dependency; its `vos.toml` is class H.
It is folded into `vos-link`'s tests when VOS-033 lands (backlog VOS-D049).

```text
cargo test -p vos-sched-contract-check      # about a minute: about 1.9 million claim evaluations
```
