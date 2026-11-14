---
category: TypeScript / JavaScript
---

# JSON.parse keeps the last duplicate key and rounds integers above 2^53, so an untrusted corpus needs its own strict reader

`JSON.parse('{"a":1,"a":2}')` returns `{a: 2}` with no error, and
`JSON.parse("9007199254740993")` returns `9007199254740992`. For a corpus that
is untrusted input and carries digests, counts and 64-bit values, either one
makes two readers disagree about the same document without anyone noticing: a
duplicated `suites` key hides a suite, a rounded `cases` count passes a product
check it should fail. Python's `json.loads` has the same duplicate-key
behaviour, which the Python lane closes with `object_pairs_hook`; JavaScript has
no such hook (the reviver runs after the damage), so the TypeScript Warrant
runner carries its own small reader: duplicate keys, non-safe integers, `-0`,
fractions, trailing data and over-deep nesting are errors, and objects have no
prototype so a `"__proto__"` key is plain data.

Do differently: if JSON is a trust boundary in TypeScript, do not call
`JSON.parse` on it. Carry 64-bit values as tagged decimal strings and parse them
with `BigInt`, never as JSON numbers.
