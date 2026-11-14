---
category: Cross-platform & Windows BUILD_windows
---

# Text-mode copies of test fixtures translate LF to CRLF on Windows and break strict line formats

The warrant-corpus tests copied the corpus with `read_text()`/`write_text()`. On Linux that is
byte-identical; on Windows `write_text` turns every `\n` into `\r\n`, so the copied `cases.tfv`
began `tfv 1\r` and the (correctly strict) TFV reader refused it: 19 Windows-only failures while
`test_the_real_corpus_loads` passed, because it reads the checked-in file directly. The same
mistake in a test that rewrote a manifest made `check` report drift.

Fix the helpers, not the parser: copy and rewrite fixtures with `read_bytes`/`write_bytes`, keep
production writers on `newline="\n"`, and add a test that the helpers preserve bytes exactly and a
test that a CRLF corpus is refused. Also guard platform impossibilities explicitly
(`os.geteuid`, non-UTF-8 file names on NTFS) with a `skipif` that states why.
