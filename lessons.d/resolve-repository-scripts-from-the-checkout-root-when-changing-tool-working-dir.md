---
category: Repo policy / workflow reminders
---

# Resolve repository scripts from the checkout root when changing tool working directories

While validating VER26 step 4, one command used the Go build-tool directory as
its working directory but kept checkout-root-relative Python/Ruff paths. Those
checks failed with missing files while the following Go test passed. A shell
command's final exit status did not establish that all preceding checks ran.

Run repository scripts from the checkout root and Go commands from the Go
module, in separately inspected calls. Stop dependent steps when their check
fails. Ruff also rejected imported `noqa: E402` suppressions because this repo
does not enable E402 and does enable RUF100; use the active lint configuration
rather than copying suppressions from another project. The corrected checks
were rerun before review and publication.
