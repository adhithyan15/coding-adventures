# Changelog

## Unreleased

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

- Re-export manifest types from the shared strict agent-manifest package.
- Emit schema-v2 manifests with complete per-channel message-schema versions;
  default zero-friction channels to version 1 and accept explicit overrides.

## 0.1.0 - 2026-08-03

- Parse Level 1 `SKILL.md` documents through the repository CommonMark AST.
- Generate typed agent manifests and deterministic JSON.
- Validate capability declarations and derive least-privilege Deno flags.
- Support optional fail-closed metadata frontmatter with safe defaults.
