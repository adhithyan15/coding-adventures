---
category: Book build and publication
---

# Check-book-compile needs a Bash-version gate before using mapfile

The full book gate materialized every book on macOS and only then failed at
`mapfile`, which does not exist in the system Bash 3.2, followed by an unbound
array error. A script that requires Bash 4 must either avoid that builtin or
check the interpreter version before expensive work and print the exact
prerequisite. Do not treat this host-shell failure as evidence that a book
itself failed to compile.
