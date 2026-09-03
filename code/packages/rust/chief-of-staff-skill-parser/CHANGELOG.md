# Changelog

## Unreleased

- Parse a required `## Tool capabilities needed` section into manifest
  `tool_capabilities`, and emit schema v4.
- Factor the section scan into one shared reader. The tools section copied the
  capabilities section's shape and inherited its bug; a third copy would have
  inherited it again.

- Parse a required `## Tools needed` section into manifest `allowed_tools`, and
  emit schema v3. `- none` declares an empty tool surface, matching how
  `## Capabilities needed` already works.
- Require the section rather than defaulting an absent one to an empty list.
  Manifest v3 requires `allowed_tools` so that "calls no tools" is declared
  instead of defaulted into; an optional section here would put the default back
  one layer up and undo that.
- Validate tool identifiers as namespaced (`artifact.write`), reject a bare
  namespace, reject duplicates, and sort them so two skills declaring the same
  tools generate byte-identical manifests.
- Accept only LITERAL Markdown in capability and tool bullets and in the section
  headings that introduce them. `inline_text` resolves an image to its alt text,
  drops raw inline HTML while keeping the text around it, and concatenates with
  no separator, so a bullet could authorize a tool that the rendered document
  never shows: `- ![&#97;dmin&#46;exec&#95;all](pixel.png)` yielded
  `admin.exec_all`, absent from the source bytes and rendered as a picture. Only
  `Text` and `CodeSpan` are accepted now.
- Count `## Tools needed` headings in nested blocks toward ambiguity. A decoy
  section inside a block quote was silently ignored, letting an author stage a
  document whose visible declaration was not the effective one.
- Enforce `MAX_ALLOWED_TOOLS` at parse time for a clearer error than the
  manifest's later rejection.

- Re-export manifest types from the shared strict agent-manifest package.
- Emit schema-v2 manifests with complete per-channel message-schema versions;
  default zero-friction channels to version 1 and accept explicit overrides.

## 0.1.0 - 2026-08-03

- Parse Level 1 `SKILL.md` documents through the repository CommonMark AST.
- Generate typed agent manifests and deterministic JSON.
- Validate capability declarations and derive least-privilege Deno flags.
- Support optional fail-closed metadata frontmatter with safe defaults.
