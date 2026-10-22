---
category: Testing & coverage
---

# Use unittest discovery for repository code script tests

Running a code-script test through the dotted name `code.scripts...` from the
Rust workspace imported Python's standard-library `code` module, which is not a
package and has no `scripts` attribute. Run these tests from the repository
root with `python3 -m unittest discover -s code/scripts/tests -p '<file>'` so
module discovery cannot collide with the standard library.
