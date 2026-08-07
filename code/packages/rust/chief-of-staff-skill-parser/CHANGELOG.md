# Changelog

## Unreleased

- Re-export manifest types from the shared strict agent-manifest package.
- Emit schema-v2 manifests with complete per-channel message-schema versions;
  default zero-friction channels to version 1 and accept explicit overrides.

## 0.1.0 - 2026-08-03

- Parse Level 1 `SKILL.md` documents through the repository CommonMark AST.
- Generate typed agent manifests and deterministic JSON.
- Validate capability declarations and derive least-privilege Deno flags.
- Support optional fail-closed metadata frontmatter with safe defaults.
