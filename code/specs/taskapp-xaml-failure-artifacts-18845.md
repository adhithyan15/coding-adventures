# TaskApp XAML failure artifacts

Issue #18845 supports the deferred-XAML investigation in #18515 and VisiCalc
epic #14267. A native validation failure currently discards the package needed
to reproduce it. Preserve diagnostic evidence without weakening release gates.

## Contract

On a failed XAML matrix job, upload only known XAML release ZIPs, a provenance
receipt with their SHA-256 hashes, and the bounded tail of the native lifecycle
smoke log if present. Use a diagnostic artifact name and seven-day retention.
An early failure with no ZIP must still produce a receipt listing no payloads.
Never enumerate application state, credentials, environment variables, or the
whole workspace. Success must continue to upload only verified release payloads.

The publication job must select only artifacts whose names end in `-release`.
It remains dependent on successful validation and all build jobs. Diagnostic
upload does not turn failure into success or provide release acceptance.

## Validation

Exercise receipt generation with a known ZIP and long smoke log, and with no
payload or log. Check hash fidelity, bounded log length, source identity, and
exclusion of unrelated files. Inspect success/failure guards and the publication
artifact filter. GitHub execution of the failure branch remains necessary to
establish hosted artifact-upload behavior.
