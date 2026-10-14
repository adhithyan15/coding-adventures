## Unreleased — bounded Railroad numeric admission

Reject nonfinite, negative and above65536 values for padding and both separation
fields through all four source dialects and both YAML/init forms. Verify exact
field errors in168 invalid cells and complete retained config in120 positive
cells. Preserve producer corpus and metadata controls. Full affected suites
and strict lint pass; no new dependencies. Miri DEFERRED/PENDING, never passed.
