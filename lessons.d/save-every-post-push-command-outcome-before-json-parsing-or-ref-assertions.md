---
category: Repo policy / workflow reminders
---

# Save every post-push command outcome before JSON parsing or ref assertions


The first pipeline publisher was withheld by independent preflight review before
execution because check_output would drop failed post-push command stdout/stderr
and exit status. Preserve the rejected helper and refusal receipt. Its replacement
captures command argv, exit, stdout and stderr before parsing, saves each parse or
assertion stage, and records exception details. A push0 with failed verification
must remain push0/null verification until a separate read-only reconciliation;
never repush to repair missing diagnostics. No production code or test gate changed.
