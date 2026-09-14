# A guard that encodes a guess about the environment fails in the environment

The apt wrapper above asserted, before installing, that an Ubuntu archive was
still configured — by grepping the source lists for `archive.ubuntu.com`,
`ports.ubuntu.com`, or `azure.archive.ubuntu.com`. It passed every local
fixture, including two I built to mimic the 24.04 and 22.04 layouts. On the
real runner it failed immediately:

```
pruned 1 vendor source list(s): microsoft-prod.list
error: no Ubuntu archive is configured after pruning.
```

Nothing was wrong with the pruning. The *guard* was wrong: it encoded my guess
about which mirror hostname and which file layout the runner image uses, and my
fixtures encoded the same guess, so they agreed with each other and neither
agreed with reality. Fixtures I write from an assumption cannot test that
assumption — they inherit it.

The fix was to stop asking a question that needs environment knowledge. The
invariant that actually matters is **"pruning left behind something we did not
prune"** — no hostnames, no layouts, no assumptions about the image. It is also
the thing the guard was really for: catching a vendor pattern widened until it
swallows the archive.

Two smaller points, both of which cost a CI round trip:

- **The error reported its conclusion, not its evidence.** "No Ubuntu archive
  is configured" says what the script decided; it does not say what it saw, so
  diagnosing it needed another run just to look. The script now prints the
  surviving sources unconditionally, on success as well as failure — one line
  that distinguishes "my patterns are too broad" from "this image is laid out
  differently".
- **Prefer an invariant over a recognizer.** "At least one source survived" is
  checkable without knowing anything about the world. "An Ubuntu archive is
  present" requires a list of what Ubuntu archives look like, and that list is
  maintained by somebody else.
