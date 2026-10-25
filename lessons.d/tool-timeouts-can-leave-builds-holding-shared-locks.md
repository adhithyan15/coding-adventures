---
category: Testing & coverage
---

# Tool timeouts can leave builds holding shared locks

Several validation commands exceeded the tool's output window but continued
running. Later checks accumulated behind the same Cargo target lock, including
a reviewer's independent check, and indiscriminate cleanup interrupted that
review too.

Run long validations in an explicitly tracked terminal session and poll that
session to completion. Before stopping a lingering compiler, inspect its
working directory and ownership rather than matching only the command name;
shared worktrees can have intentionally concurrent reviewers.
