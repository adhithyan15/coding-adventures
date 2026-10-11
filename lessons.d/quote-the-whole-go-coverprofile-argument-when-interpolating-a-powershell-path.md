---
category: Testing & coverage
---

# Quote the whole Go coverprofile argument when interpolating a PowerShell path

Passing `-coverprofile=$coverageFile` directly to `go test` from PowerShell
created an untracked file literally named `$coverageFile` in the package
directory instead of using the previously assigned temp path. The focused
coverage run itself passed, but the artifact had to be resolved and removed
explicitly before commit. Pass the entire native argument as an interpolated
string, for example `"-coverprofile=$coverageFile"`, then verify the expected
output file exists and the package worktree remains clean.
