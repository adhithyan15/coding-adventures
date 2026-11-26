## 0.1.2

- Security review round 2.
- The shrink budget is an `AsyncLocal` charged under a lock: it flows into child tasks and
  async continuations. `Runner.Check`, `Gen.Shrink`, `Shrink.ShrinkList` and `Shrink.ShrinkBytes`
  open an isolated budget (a re-entrant verdict is not starved by, and does not drain, the outer
  shrink); `Gen.ShrinkNested` joins and is used by tuples, lists and filters.
- Initial copies (`ShrinkSequence`, `ShrinkList`, `Bytes` list conversion) are charged first and
  skipped, returning the input, once the budget is spent.
- Docs: witness types must have a bounded, shallow `ToString`; candidate lists are fresh but
  their elements are not cloned.
