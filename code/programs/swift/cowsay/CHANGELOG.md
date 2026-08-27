# Changelog

## 0.1.0

- add the Swift port of `cowsay`, parsing `code/specs/cowsay.json` via
  `CliBuilder` and rendering `code/specs/cows/*.cow` templates
- tenth program in the repository to render through `PaintVmAscii`
  (after the C#, F#, Perl, Haskell, Java, Kotlin, and Dart ports): the
  composed bubble+cow text is converted into a `PaintScene` of `glyphRun`
  instructions and rendered via `render` instead of being printed directly
  (see `code/specs/cowsay-paintvm-pipeline.md`)
- this is also the PR that built `PaintVmAscii` from scratch, implementing
  the full P2D02 contract (rect/line/glyph_run/group/clip/layer) — and
  that converted `PaintInstructions`' `PaintInstruction` from a
  rect-only type alias into a real sum type in the first place (unlike
  every other language in this rollout, Swift's package wasn't there yet)
- support eyes/tongue/cowfile/nowrap/width/think flags and the eight mood
  shortcuts (`--borg`, `--dead`, `--greedy`, `--paranoid`, `--stoned`,
  `--tired`, `--wired`, `--youthful`)
- support `--list` to enumerate available `.cow` files
- `loadCow` validates the user-supplied `-f`/`--file` flag against path
  traversal and rooted-path overrides before reading a file, mirroring the
  fix applied to every other port's `loadCow` after `/security-review` —
  and deliberately does NOT use `URL(fileURLWithPath:).lastPathComponent`
  for basename extraction, since that resolves a relative input against
  the current working directory rather than treating it as a literal
  string (see `basenameOf`'s doc comment in `Sources/Cowsay/Cowsay.swift`)
- output is written via `FileHandle.write(Data)` rather than `print()`,
  which is never subject to platform newline translation — LF-only output
  without needing the explicit workaround the JVM/Dart ports needed for
  the same guarantee
- `CommandLine.arguments` is passed straight through to `CliBuilder`'s
  `Parser` with no placeholder prepended, since (unlike Kotlin/Java/Dart)
  it already includes the executable path at index 0 — see README "A note
  on argv"
