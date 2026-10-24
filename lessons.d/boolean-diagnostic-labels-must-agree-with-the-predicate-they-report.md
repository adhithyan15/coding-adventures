---
category: Repo policy / workflow reminders
---

# Boolean diagnostic labels must agree with the predicate they report

A cleanup verifier correctly asserted that its target path no longer existed,
but its final diagnostic evaluated that same absence predicate and printed
`no` under the label `target_removed`. The destructive operation and assertion
were correct, yet the inverted summary made successful evidence look like a
failed cleanup and required a second verification.

Name the predicate for the positive claim being printed, map its true branch
to `true` or `yes`, and keep the assertion beside the diagnostic. For example,
`if [ ! -e "$target" ]; then printf 'target_removed=true\n'; else ...; fi`
reports the same fact it validates. Treat human-readable status output as part
of the verification contract, not decoration after the real checks.
