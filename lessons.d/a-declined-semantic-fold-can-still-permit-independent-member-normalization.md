---
category: Testing & coverage
---

# A declined semantic fold can still permit independent member normalization

The array-length regression initially required `changed == false` for both
`[g()].length` and `[g()]["length"]`. The computed spelling legitimately
normalizes its string key to dot access after the count fold declines, so that
assertion rejected existing correct behavior. Require the member expression to
survive and forbid only the outer array-count contribution. Keep separate
unchanged assertions for the dot spelling. Inspect the actual failure before
labeling every failed new assertion as proof of the targeted defect.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
