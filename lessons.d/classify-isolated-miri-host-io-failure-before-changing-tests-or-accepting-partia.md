---
category: Testing & coverage
---

# Classify isolated Miri host IO failure before changing tests or accepting partial success

The final-source CV04a local script exited101 after Miri rejected
host_read_byte_returns_value_in_dest: stdin reads are unsupported under host
isolation. This was a runner capability failure, not an undefined-behavior report.
The complete Twig library did not pass: its harness aborted, despite runtime-core,
dynval and four Twig doctests completing successfully. A zero parsed failed-test
count cannot override a failing process/target. Preserve the complete failed log.

Keep the host/read_byte success and integer-result assertions intact. Specify and
verify controlled noninteractive stdin plus the supported host-access Miri mode
before changing the canonical runner. Scope host access to the VM suite, retain
the substrate's existing isolation and all UB detection, and require a full new
terminal script pass. Do not skip the offending test or accept a partial run.

Runner controls observed both the missing flag and consumed invoking stdin before
repair. The same actual host test then passed with host access and closed stdin.
The Python fixture initially failed Ruff import ordering; use the formatter's
exact result instead of guessing blank-line policy. Keep subprocess exit codes
separate: a later shell check succeeding does not make an earlier failed lint or
an aborted Miri target green. Main-merge whitespace warnings were in unchanged
upstream Markdown hard breaks; candidate-versus-main diff checks remain clean.
