---
category: CI & GitHub Actions
---

# A near-threshold browser score needs an unchanged-head rerun before a code diagnosis

Run 38204943619 built and tested every Forme product on Windows, then failed
the landing-page browser gate: performance median 0.94 against the unchanged
0.95 requirement. The gate passed on the previous head. This revision changes
corpus test fixtures and documentation; it does not change landing-page assets
or the browser performance gate.

Keep the threshold intact and inspect the gate's sampling and compared assets.
An unchanged-head rerun is needed before attributing this difference to code.
GitHub rejected an immediate job rerun with HTTP 403 because the containing
workflow is still running. Wait for that workflow to finish, then rerun the
failed check and require its actual result. A near-threshold failure alone is
not evidence that the dependencies or product are defective, nor proof that the
failure is transient.
