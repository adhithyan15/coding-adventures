---
category: TypeScript / JavaScript
---

# Hand-rolled parsers walking attacker-controlled keys MUST defend against prototype pollution

`target[seg] = value` resolves `target["__proto__"]` to `Object.prototype` and the subsequent write pollutes the global prototype. Two-layer fix: (1) reject `__proto__`/`constructor`/`prototype` segments at the lex layer with an explicit denylist; (2) construct every internal table as `Object.create(null)` so even if the denylist is bypassed there's no prototype chain to walk. Caught in `forme-manifest` parser, retroactively applied to the JSON Schema validator in `forme-pipeline-config`. Test: `Object.keys(Object.prototype).length` before/after parsing must be equal.
