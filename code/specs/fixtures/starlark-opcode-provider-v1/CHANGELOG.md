# Changelog

## Unreleased

- Specify STP01 `standalone-provider-v1` as an inert component compatibility
  profile, separate from the future PY03 restricted runtime architecture.
- Freeze 61 assigned byte values, 195 holes across all 256 decode outcomes,
  34 group-specific operator entries and eight negative lookup probes.
- Add a closed structural schema and an independent literal validator that
  rejects coordinated table corruption, ambiguous JSON, invalid scalars,
  coercible integer representations and transport/depth violations.
- Add exhaustive decode corruption, table/schema, loader and CLI tests, and
  invoke both the oracle and its tests in the existing unconditional metadata
  gate. The validator checks zero native adapters; lane adoption, collection
  ownership tests, Python implementation and Ruby applicability stay separate.
