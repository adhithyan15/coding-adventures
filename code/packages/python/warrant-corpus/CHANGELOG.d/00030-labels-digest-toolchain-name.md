## Unreleased

- The `domain-labels.json` digest now binds the manifest's own `toolchain.name`
  instead of the literal `python`, so `validate-manifest` and `check` accept the
  label sidecar of another lane's manifest (the TypeScript lane's toolchain is
  `node`). The name is still part of the digest (swapping runtimes is drift);
  only `toolchain.version` is ignored. Python output is byte-identical.
