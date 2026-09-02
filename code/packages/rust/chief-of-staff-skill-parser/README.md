# chief-of-staff-skill-parser

Fail-closed parser for D18 Level 1 `SKILL.md` agents. It reuses the repository
CommonMark AST and the shared `chief-of-staff-agent-manifest` contract, produces
a typed schema-shaped manifest, and derives sorted least-privilege Deno flags.

An H1 title, descriptive first paragraph, and `## Capabilities needed` and
`## Tools needed` sections are required. The issue's zero-frontmatter form is valid; identity and safe
defaults are inferred. Optional `---` frontmatter accepts only `agent`,
`description`, `privilege_tier`, `reads`, `writes`,
`message_schema_versions`, and `restart_policy`. The parser emits manifest v3
and defaults every declared channel to payload schema version 1. Authors can
override all channel versions with a complete list such as
`message_schema_versions: [requests=1, responses=2]`; missing, extra, duplicate,
malformed, or zero-valued entries fail closed.

Capability bullets use `category:action:target`, optionally followed by
` | justification`. Use `- none` for an explicit empty capability profile.

Tool bullets are D18D tool identifiers, one per line, and become manifest
`allowed_tools`. Use `- none` for an explicit empty tool surface. Identifiers
must be namespaced (`artifact.write`, never `artifact`): a bare namespace names
no tool and would invite prefix matching in the broker, which is how one
declared tool becomes a whole namespace. Tools are sorted, so two skills
declaring the same set generate byte-identical manifests.

The section is **required**, not optional. Manifest v3 requires `allowed_tools`
so that "calls no tools" is declared rather than defaulted into; letting an
absent section mean an empty list would move that default one layer up and undo
it.

```markdown
## Tools needed
- artifact.write
- context.append_entry
```

## Validation

```sh
sh chief-of-staff-skill-parser/BUILD
```
