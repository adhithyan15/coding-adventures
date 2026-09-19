---
category: Testing & coverage
---

# Generated CLI goldens must be rewritten from process stdout to preserve terminal blank lines

`closurec --help_markdown` intentionally ends with two newline bytes. Replacing
only its version line with a patch left one terminal newline, so every visible
line looked correct while the byte-exact integration test failed. Regenerate a
generated golden by capturing the executable's stdout and writing those exact
UTF-8 bytes without a BOM; then compare lengths, the first differing byte, and
the escaped tail before rerunning the test. Do not hand-edit a generated
snapshot when trailing blank lines are part of its contract.
