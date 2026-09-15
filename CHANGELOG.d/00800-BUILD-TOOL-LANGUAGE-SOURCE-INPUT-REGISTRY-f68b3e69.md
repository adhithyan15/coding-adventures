### Build-tool language source-input registry

- Replaced the Swift build tool's hand-maintained source/metadata allowlists
  with an immutable generated projection of the full neutral registry. Swift
  now enforces every package-local selector role and scope, exact Engram paths,
  unknown-language rejection, language/root binding, portable identity
  validation, pre-sort candidate limits, canonical registry-digest evidence,
  and finite source bounds through the same production matcher consumed by four
  neutral cases.
  Retained-handle reads now stop at the snapshotted byte count and reject an
  additional probe byte, closing concurrent POSIX growth before allocation.
  Repository-relative boundary adoption remains an exact separate projection
  rather than being approximated with broader package authority.
- Registered the exact Engram WASM BUILD inputs for that package root only: the
  smoke script, its imported host module, and the checked-in WebAssembly bytes.
  Added a language-neutral exact-path fixture and tracked Git-blob projection
  while keeping other Rust packages, `.mjs`, `.js`, `.wasm`, generated-output,
  and directory-wide authority closed.
- Added a closed versioned source-input registry for all 23 canonical build-
  tool language keys. It distinguishes recursive sources and metadata,
  package-root exact and variable manifests, fixed relative inputs, and
  path-scoped native companions or resources while retaining universal BUILD
  fronts and root-only `required_capabilities.json`.
- Added strict schema and semantic validation for canonical UTF-8 order,
  bounded selector counts, unsafe and generated paths, NFC/full-casefold
  collisions, explicit case aliases, inclusion-only scoped classifications,
  scoped/global and fixed-path match overlap, hostile candidate identities,
  and a domain-separated registry digest. Source fixtures now pin only a
  language and that digest instead of supplying their own allowlists.
- Added bounded tracked-input coverage for exact BUILD-invoked C, C++, .NET,
  Dart, Go, Haskell, and Swift scripts plus reviewed Mosaic and Rust package
  build scripts; shared Unix command
  specifications; Go CommonMark metadata; Mosaic hosts, tests, and tokens;
  Rust host templates and fixtures; nested .NET test and WinUI projects;
  Kotlin Android/wrapper inputs; and reviewed Dart, Elixir, F#, Go, Lua, Perl,
  Python, Ruby, Rust, and TypeScript test or resource families. Dart hashes
  only the two tracked Android Gradle properties, never ignored signing or SDK
  properties. A repository-projection regression proves representative
  tracked inputs are selected through the production-neutral registry.
- Audited every production build-tool engine and registered exact-registry
  adoption work for all eleven engines, keeping those serial engine changes
  outside this neutral corpus tranche. Shared ancestor inputs and tracked
  vendor files hidden by generated-component pruning are explicitly owned by
  a dependent exact-boundary contract rather than unsafe broad selectors.

