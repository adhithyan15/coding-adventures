---
category: TypeScript / JavaScript
---

# `Object.prototype.hasOwnProperty.call(obj, key)` not `key in obj` when checking attacker-controlled keys

`in` walks the prototype chain, so `"toString" in {}` is `true` (via `Object.prototype.toString`). In a JSON Schema validator this means `required: ["toString"]` passes vacuously and `additionalProperties: false` with empty `properties: {}` accepts `{ toString: "x" }`. Bypass found in `forme-pipeline-config`; fix uses a `hasOwn(obj, key)` helper at every `key in` call site that touches user data.
