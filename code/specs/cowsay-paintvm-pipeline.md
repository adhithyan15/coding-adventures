# Cowsay on PaintVM-ASCII

A design decision and technical contract for routing `cowsay` ports through
the `paint-vm-ascii` backend, resolving the open question raised in issue
#1611.

---

## 1. Background

`cowsay` ships on six languages today (python, go, rust, typescript, ruby,
elixir), each hand-rolling its own bubble-border and cow-template rendering
directly to a string that gets printed to stdout. Issue #1611 asked whether
the remaining ports should keep doing that, or route through the existing
`paint-vm-ascii` backend (`P2D02-paint-vm-ascii.md`), which already exists
as the text-mode target of the shared PaintInstructions IR
(`P2D00-paint-instructions.md`):

```
cli_builder (parse argv)
  -> cowsay text formatting (word wrap, bubble border, cow template)
  -> paint-instructions (PaintScene of PaintGlyphRun instructions)
  -> paint-vm-ascii (render to a character-grid string)
  -> terminal
```

**Decision: every new `cowsay` port routes through `paint-vm-ascii`.** This
makes `paint-vm-ascii` a real, exercised backend instead of a set of
library-only packages nobody calls, and it means the bubble/cow rendering
logic that already exists in seven other languages' `paint-vm-ascii`
packages doesn't get re-invented a ninth and tenth time as hand-rolled string
code.

Whether the existing six ports get migrated onto the same pipeline, and
whether they get the `BUILD`/tests/README/CHANGELOG they're currently
missing, is an explicit follow-up — not part of this decision.

---

## 2. Corrected coverage matrix

Issue #1611's own coverage matrix was stale by the time this spec was
written. The state below was verified directly against the repository
(`ls code/packages/*/paint-vm-ascii`, `code/packages/*/paint-instructions`,
`code/programs/*/cowsay`, and a grep for `glyph_run`/`GlyphRun` inside each
`paint-vm-ascii` package):

| Language   | cowsay | paint-instructions | paint-vm-ascii | paint-vm-ascii implements `glyph_run` |
|------------|:------:|:-------------------:|:---------------:|:---------------------------------------:|
| python     | ✅     | ✅                   | ✅               | ❌ (rect-only)                          |
| go         | ✅     | ✅                   | ✅               | ❌ (rect-only)                          |
| rust       | ✅     | ✅                   | ✅               | ✅                                       |
| typescript | ✅     | ✅                   | ✅               | ✅                                       |
| ruby       | ✅     | ❌                   | ❌               | —                                        |
| elixir     | ✅     | ❌                   | ❌               | —                                        |
| csharp     | ❌     | ✅                   | ✅               | ✅                                       |
| fsharp     | ❌     | ✅                   | ✅               | ✅                                       |
| perl       | ❌     | ✅                   | ✅               | ❌ (rect-only)                          |
| haskell    | ❌     | ✅                   | ✅               | ❌ (rect-only)                          |
| java       | ❌     | ✅                   | ❌               | —                                        |
| kotlin     | ❌     | ✅                   | ❌               | —                                        |
| dart       | ❌     | ✅                   | ❌               | —                                        |
| lua        | ❌     | ❌                   | ❌               | —                                        |
| swift      | ❌     | ✅                   | ❌               | —                                        |

Notes on the issue's original matrix:

- **"dotnet" is not a distinct language.** `code/programs/dotnet/` only holds
  the shared `hello-world-csharp` / `hello-world-fsharp` / build-tool
  bootstrap programs. Real csharp/fsharp feature programs (including this
  one) live under `code/programs/csharp/` and `code/programs/fsharp/`
  directly, same as every other language.
- **"wasm" is explicitly out of scope**, per the issue's own text (terminal
  output through a wasm program is a separate design problem).
- That leaves **9 real target languages**: lua, perl, swift, haskell, csharp,
  fsharp, java, kotlin, dart.
- No program anywhere in the repository currently consumes any
  `paint-vm-ascii` package. This is a first-of-its-kind wiring.

---

## 3. The technical contract

Cowsay's own formatting logic — word wrap, eyes/tongue/mode substitution,
bubble border characters (`_`/`-`/`<`/`>`/`\`/`/`/`|`/`(`/`)`), and `.cow`
template substitution (`$eyes`, `$tongue`, `$thoughts`) — is plain text
formatting, language-agnostic, and does not change. Port it faithfully from
the existing reference implementation, `code/programs/go/cowsay/main.go`
(`wrapText`, `formatBubble`, `loadCow`, and the mode-flag-to-eyes/tongue
table).

What changes is the last step: instead of printing the composed text block
directly, build a `PaintScene` from it and render that scene through
`paint-vm-ascii`.

### 3.1 Text lines to glyph placements

Given the composed bubble+cow text as a list of lines:

1. For each line at row index `r`, for each character at column index `c`
   that is not a space, create a glyph placement at
   `x = c * scale_x, y = r * scale_y`, with `glyph_id` set to the character's
   Unicode code point. Skip space characters — unaddressed cells default to
   blank in the ASCII backend's character grid.
2. Use `paint-vm-ascii`'s default scale factors, `scale_x = 8, scale_y = 16`
   (`P2D02-paint-vm-ascii.md`), unless a port's convenience API defaults to
   something else — always pass the scale explicitly if there's any doubt.

**Important deviation from the general `PaintGlyphRun` contract:** per
`P2D00-paint-instructions.md`, `glyph_id` is normally a font-internal glyph
index (not a Unicode code point), resolved through a real font's `cmap`
table. `P2D02-paint-vm-ascii.md` explicitly relaxes this for the ASCII
backend only: "`glyph_run` is rendered by converting each `glyph_id` into a
Unicode scalar value ... This backend is best suited to glyph runs whose
`glyph_id` values are already ordinary Unicode code points." Every cowsay
port must rely on this backend-specific relaxation — do not attempt real
glyph-ID resolution for a terminal target.

### 3.2 Glyph placements to a scene

Group the placements into one or more `PaintGlyphRun` instructions (one per
line is simplest and matches the natural row-major structure of the text).
`font_ref`, `font_size`, and `fill` are required fields on `PaintGlyphRun`
but are explicitly ignored by the ASCII backend — any placeholder value
(e.g. `font_ref: "terminal-mono"`) is correct.

Wrap the glyph runs in a `PaintScene`:

```
{
  width: (longest line length) * scale_x,
  height: (line count) * scale_y,
  background: "transparent",
  instructions: [ <PaintGlyphRun>, <PaintGlyphRun>, ... ]
}
```

### 3.3 Rendering

Call the language's `paint-vm-ascii` render entry point (e.g. C#'s
`PaintVmAscii.RenderToAscii(scene)`, or the equivalent `render(scene,
options?)` convenience function called out in `P2D02-paint-vm-ascii.md` §
"Public API") and print the returned string.

### 3.4 Output identity requirement

`paint-vm-ascii` trims trailing spaces on each line and trailing blank lines
at the end of the document (spec-mandated). Cowsay's own output has never
depended on trailing whitespace for visible content, so routing through this
pipeline must reproduce **byte-identical** output to the direct-print
approach used by the existing six ports. Any port added under this contract
should be checked against another existing port for the same flags and
message, and the two outputs must match exactly.

---

## 4. Rollout phases

Each phase is its own set of small, focused PRs — never bundled into one
PR per the repo's stated convention (issue #1611's own "suggested next
steps," and the general small-PR discipline in `CLAUDE.md`).

- **Phase 1 (this PR)** — **C#**. `code/packages/csharp/paint-vm-ascii` is
  the only fully spec-compliant implementation found (rect, line, glyph_run,
  group, clip, layer all registered against a real dispatch-table VM). C#
  also already has `cli-builder` and `paint-instructions`. This phase needs
  zero backend work — it exists purely to prove the pipeline described in
  §3 holds together end-to-end, before investing in the backend work the
  later phases need.
- **Phase 2** — **F#** (already has full `glyph_run` support — same pattern
  as C#, no backend work needed); **Perl** and **Haskell** (need `glyph_run`
  added to their existing rect-only `paint-vm-ascii` packages first, then
  cowsay on top).
- **Phase 3** — **Java, Kotlin, Dart, Swift**. These have `paint-instructions`
  already (Swift's is `PaintInstructions`, following Swift's PascalCase
  package-naming convention) but no `paint-vm-ascii` package at all — build
  one from scratch implementing the full `P2D02-paint-vm-ascii.md` contract
  (not just rect), then add cowsay.
- **Phase 4** — **Lua**. Has neither `paint-instructions` nor
  `paint-vm-ascii` — both packages need to be built from scratch before
  cowsay can land. The only language in this position.

---

## 5. Explicitly out of scope

- Migrating the existing six hand-rolled `cowsay` ports onto this pipeline.
  This is a real question (raised in issue #1611 itself) but is independent
  of adding the missing languages and is left for a future decision.
- Giving the existing six `cowsay` programs the `BUILD`/tests/README/
  CHANGELOG they currently lack. They have none today — none of them are
  wired into CI. This is a pre-existing gap unrelated to the languages this
  spec adds, called out here so it isn't mistaken for something this spec
  fixed.
