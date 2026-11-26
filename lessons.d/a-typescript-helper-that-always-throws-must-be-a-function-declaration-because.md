---
category: TypeScript / JavaScript
---

# A TypeScript helper that always throws must be a function declaration, because a const arrow function typed never does not narrow control flow

**What went wrong.** The v2 runner's refusal helper was written `const fail = (code, why): never => { throw ... }`. After `if (a.ty.k !== "Bytes" || b.ty.k !== "Bytes") fail(...)`, `tsc` still treated `a.ty` as the wide `Ty` union and reported `Property 'n' does not exist on type 'Ty'` on the very next line. TypeScript narrows after a call to a function that returns `never` only when the call's target has an **explicit** `never` type annotation that it can see: a function declaration, or a `const` with a written function type. An arrow function whose return type is merely annotated on its own signature is not enough.

**The fix.** Write the helper as `function fail(code: KitCode, why: string): never { throw new KitError(code, why); }`. Every `if (bad) fail(...)` guard then narrows what follows, with no `as` casts and no `else` branches.

**What to do differently.** A guard helper that always throws (`fail`, `refuse`, `need`) is a function declaration, or an `asserts condition` function declaration like `need` in this package's `errors.ts`. If narrowing after a guard stops working, check how the helper is declared before adding casts.
