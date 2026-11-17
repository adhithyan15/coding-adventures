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

**Receipt (PR #19586, `warrant-oracle`):** the Windows job failed `test_a_stale_accept_blind_line_fails_the_profile_check` with `'corpus' == 'generation'`. The test rewrote `corpus.profile` with `Path.write_text`, so Windows wrote CRLF, and the profile reader correctly refused the CR as a `corpus` error before the generation check the test was meant to reach. Other writes in that package passed on Windows for the same wrong reason. The fix routes every parsed fixture through `write_lf` in `tests/conftest.py` (`write_bytes(text.encode("utf-8"))`), reads them with an explicit `encoding="utf-8"`, and adds a regression test that a profile written with `newline="\r\n"` is refused as `corpus`, so the LF requirement is pinned.
