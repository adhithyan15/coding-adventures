---
category: Security boundaries
---

# A required capabilities manifest must use the repository schema before security review

The first Forme desktop manifest used an ad hoc string list and a
`schema_version` field, so it neither validated against
`code/specs/schemas/required_capabilities.schema.json` nor described the
native leaf's actual filesystem, loopback, process, FFI, environment, and
clock authority. Read the repository schema and one current non-empty example
before authoring a new manifest. Use exact capability objects with scoped
targets and reviewable justifications, then validate the JSON structure before
requesting the mandatory security review.
