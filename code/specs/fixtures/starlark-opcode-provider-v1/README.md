# Standalone Starlark opcode-provider v1

[STP01](../../STP01-starlark-opcode-provider.md) defines this component profile.
The corpus pins 61 assigned byte values, all 256 decode outcomes (195 holes),
34 operator entries and eight negative lookup probes: 359 ordered records.
These are reviewed compatibility data, not outputs generated from a provider.

Run the strict independent validator from the repository root:

```sh
python code/scripts/starlark_opcode_provider_conformance.py
python -m unittest discover -s code/scripts/tests -p test_starlark_opcode_provider_conformance.py
```

The JSON Schema closes structural fields; the standard-library validator also
checks normative literal values, duplicate keys, exact types, UTF-8/scalars,
131072-byte transport and depth-eight bounds. A high-nibble hole stays absent.
Unknown operators are group-specific and never normalized.

No language is credited with a committed shared-corpus reader by this foundation.
Native implementations and ownership tests remain separate adoption work. Inert
module/print labels do not execute, import, print or authorize anything. Python's
stub, Ruby's source facade and PY03's future restricted mode are separate scopes.
