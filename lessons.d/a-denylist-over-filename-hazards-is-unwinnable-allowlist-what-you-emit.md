# A denylist over filename hazards is unwinnable; allowlist what you emit

Four rounds of security review on one archiving function. Each round found one
more filename-equivalence rule the previous fix had not modelled:

1. Backslash — legal in a POSIX filename, a separator to some Windows
   extractors, so `..\..\..\evil.exe` normalises to one harmless in-payload
   component under the rules we validated with.
2. Case folding — `source_commit` and `SOURCE_COMMIT` are two members on the
   Linux runner and one file on the downloader's machine, where the second
   silently wins.
3. Win32 stripping **trailing dots and spaces** — `index.html.` and
   `index.html`, same trick, no symlink and no code execution needed.
4. Still waiting: NTFS `:` alternate data streams, reserved device names
   (`CON`, `NUL`, `COM1`), and HFS/APFS Unicode normalisation, where NFC and
   NFD are one file.

That is not a run of unlucky misses. The rule set is defined by **other
people's software**, so it only ever grows, and every miss is silent — the
archive lists two plausible names and the reader gets one file.

The inputs, by contrast, are a closed set: Vite emits hashed ASCII, jpackage
emits ASCII plus the occasional space. Naming the safe shapes instead makes
every one of those rules *unreachable* rather than individually patched:

```python
SAFE_COMPONENT = re.compile(r"[A-Za-z0-9_.][A-Za-z0-9._+ -]*")
```

Before tightening it I checked it against the filenames the real payloads
actually contain rather than guessing — every one passes, including
`_CodeSignature`, `.jpackage.xml`, and `ASSEMBLY_EXCEPTION`. The allowlist's
cost is false positives; those fail loudly at release time, which is the right
direction, and it can be widened once with evidence instead of tightened
repeatedly under attack.

The general rule: **when a check must predict how someone else's software will
interpret your output, enumerate what you produce, not what they might
mishandle.**

Two smaller things from the same review:

- **A comment that overstates what a check proves is a maintenance trap.** I
  wrote that lexical normalisation was "sound here because `rglob` never
  descends into a symlink". True of components *above* the link, false of the
  target, which can traverse one — `normpath` collapses `L/..` to L's parent,
  not L's target's parent. The physical `resolve()` caught it; the comment
  invited someone to delete that as redundant.
- **Test on a filesystem with the property you are testing.** The case-fold
  guard could not even construct its fixture on macOS and skipped. A skipped
  test reports green while checking nothing, so I created a case-sensitive APFS
  volume, pointed `TMPDIR` at it, and ran the suite there: 58 tests, zero
  skipped.
