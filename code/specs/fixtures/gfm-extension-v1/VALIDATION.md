# Portable GFM extension v1 validation

This receipt covers the neutral fixture and validator on branch source
`4d338a318c`. It does not grant native parser adoption credit or qualify a
missing language port.

## Passing checks

- The strict validator accepts all sixteen ordered cases.
- Eleven independent tests pass. They corrupt each case's source, AST, and HTML
  separately; reject duplicate keys, ambiguous JSON numbers, invalid UTF-8 and
  Unicode scalars, excessive depth/size/work, aliased or cyclic object graphs,
  reordered cases, open node fields, malformed table/task/fence structure,
  host/ECMAScript whitespace drift, and changed normative literals.
- Measured validator coverage is 248/248 statements and 144/144 branches.
- The Draft 2020-12 schema is valid and accepts the normative document with no
  errors.
- Ruff lint and format checks and Python bytecode compilation pass.
- The package-parity reporter and its ten tests pass: 1,535 established
  identities, 4,817 slots, zero canonical collisions, and zero unknown buckets.
- All 1,465 lesson shards validate.
- The real Go build tool evaluates 86 Starlark BUILD files and discovers 5,354
  packages. Because this tranche changes only neutral specs and repository
  validation scripts, it correctly reports no changed native package roots.

## Scope limits

The TypeScript reference parser's existing 743-test suite passes and was used
to cross-check the independently written literals during preparation. It does
not yet contain a committed reader for this new corpus, so TypeScript receives
no adoption credit in this tranche. C#, F#, Haskell, Swift, Java, Kotlin, and
Dart remain separate backlog owners. No fence payload was executed and no
filesystem, network, browser, plugin, or diagram authority was introduced.
