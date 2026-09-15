# `Path.resolve()` disagrees with itself across platforms on a symlink loop

CI caught a test that passed locally for the wrong reason. Given `a -> b` and
`b -> a`, non-strict `Path.resolve()` raises `RuntimeError` on macOS and
silently returns on Linux. I had written the guard as `except (OSError,
RuntimeError)` and asserted only that *some* `ValueError` came out, so on my
machine it passed via macOS's exception and on the Linux runner nothing was
raised at all.

The real defect was not the test. It was that the same payload would have been
**refused on the macOS runner and published from the Linux one** — and a check
whose verdict depends on where it ran is worse than no check, because it looks
like one. The fix is to walk the symlink chain by hand with a hop limit and a
seen-set, which is pure Python and gives the same answer everywhere. A dangling
link still has to be accepted (jpackage bundles contain them), so the walk
stops at anything that is not a symlink, which includes nothing at all.

Two generalisations:

- **Assert the message, not just the exception type**, when more than one code
  path can raise that type. `assertRaises(ValueError)` was satisfied by a
  completely different failure. Pinning the string is what proved my check was
  the one firing — and mutation-testing it showed macOS's `resolve()` quietly
  standing in for the guard I thought I was testing.
- **Anything whose answer comes from the OS needs the same answer on every OS
  you ship from.** Worth an explicit look when a repo builds the same artifact
  on three runners: `nm` missing on Windows, `zip` missing on Windows, and now
  `resolve()` behaving differently on macOS were all the same failure shape in
  one afternoon.
