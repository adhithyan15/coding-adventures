---
category: Repo policy / workflow reminders
---

# Expand Windows file sets before passing them to ripgrep

A Windows security scan passed `code/scripts/tests/test_der_asn1*` directly to
`rg`. PowerShell did not expand that wildcard into paths, and ripgrep rejected
the literal asterisk as an invalid Windows filename. Search the containing
directory with `--glob`, or expand the file list explicitly, before supplying
Windows paths to ripgrep. This recurred minutes later with `*.go`; treat a
literal wildcard in any explicit Windows path argument as a stop condition and
rewrite it as `rg --glob '*.go' PATTERN DIRECTORY` before execution.
