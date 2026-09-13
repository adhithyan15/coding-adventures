---
category: TypeScript / JavaScript
---

# Vitest stubbing `crypto`

must include `getRandomValues` and `subtle` from `node:crypto.webcrypto`, bound via arrow function (NOT `{...webcrypto}` — methods are on prototype and need internal-slot `this`):
```ts
vi.stubGlobal("crypto", {
  randomUUID: () => "mock-uuid",
  getRandomValues: (b) => webcrypto.getRandomValues(b),
  subtle: webcrypto.subtle,
});
```
