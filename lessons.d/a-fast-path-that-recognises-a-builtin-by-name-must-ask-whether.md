---
category: Compiler / VM / language pipeline
---

# A fast path that recognises a builtin by name must ask whether the name still resolves to the builtin, not only whether a local shadows it

Twig's IR compiler has typed fast paths that spot `string-append`,
`substring`, `string-length` and `+ - * /` by name and emit typed ops
(`str_len`, `str_concat`, `str_index`) that native backends trust. The fix
for #18435 made them skip a name bound by a *local*. But the call itself
was resolved by a longer chain: local, then top-level function or extern,
then top-level value, and only then the builtin. So after
`(define (string-append a b) 5)` the call ran the user's function while the
fast path around it typed the result as a string: `str_len` on 5, which is
type confusion on a native backend (#18438).

Write one predicate, "does this name mean the builtin here?", that walks
the same resolution chain the call does. Every site that treats a name as a
builtin asks it: the `call_builtin` arm, each fast path, and every analysis
that derives type evidence from a call's shape. A checker that reads a
guard's meaning from its operator's name (for example narrowing on
`(< x 128)`) has the same trap. When you fix a shadowing bug at one level
(locals), grep for every name match against a builtin and check the other
levels (globals, externs, imports) at the same time.
