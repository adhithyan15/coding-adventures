# chief-of-staff-skill-parser

Fail-closed parser for D18 Level 1 `SKILL.md` agents. It reuses the repository
CommonMark AST and the shared `chief-of-staff-agent-manifest` contract, produces
a typed schema-shaped manifest, and derives sorted least-privilege Deno flags.

An H1 title, descriptive first paragraph, and `## Capabilities needed` section
are required. The issue's zero-frontmatter form is valid; identity and safe
defaults are inferred. Optional `---` frontmatter accepts only `agent`,
`description`, `privilege_tier`, `reads`, `writes`,
`message_schema_versions`, and `restart_policy`. The parser emits manifest v2
and defaults every declared channel to payload schema version 1. Authors can
override all channel versions with a complete list such as
`message_schema_versions: [requests=1, responses=2]`; missing, extra, duplicate,
malformed, or zero-valued entries fail closed.

Capability bullets use `category:action:target`, optionally followed by
` | justification`. Use `- none` for an explicit empty capability profile.

## Validation

```sh
sh chief-of-staff-skill-parser/BUILD
```
