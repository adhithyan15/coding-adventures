---
category: TypeScript / JavaScript
---

# A table keyed by a name read from input must be a Map, never a plain object, or constructor and toString are found in it

**What went wrong.** The TypeScript v2 runner (VER04r, S7a) kept its scalar type names, descriptor kinds and leaf sizes in object literals (`const SCALARS: Record<string, Ty> = { Bool: ..., U8: ... }`) and looked a name up with `SCALARS[word]` or `name in SCALARS`. A name that comes from the corpus is hostile input, and `constructor`, `toString`, `valueOf`, `hasOwnProperty` and `isPrototypeOf` are *found* on every plain object: `parseType("toString")` returned a function as if it were a type, a legal error type called `constructor` was refused as "not usable", and a descriptor kind `constructor` read as a level instead of an unknown kind. The kit pinned none of it, because no kit case names a type `toString`; the review found it by trying.

**The fix.** Make every table that is indexed by an untrusted name a `Map` (`SCALARS.get(word)`, `SCALARS.has(name)`), or test `Object.hasOwn(table, name)` before reading. Add a test that feeds the six names (`constructor`, `toString`, `valueOf`, `hasOwnProperty`, `__proto__`, `isPrototypeOf`) through every door that reads a name: a type, an error type, a record, a descriptor kind.

**What to do differently.** In input-handling code a plain object is a record of *known* keys only. If the key is read from the data, use a `Map` or a `Set`, and grep a new module for `Record<string,` and ` in ` lookups before it ships.
