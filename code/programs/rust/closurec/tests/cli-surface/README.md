# Closure CLI surface audit

`v20260915-audit.json` is the deterministic, offline compatibility ledger for
`closurec`'s command-line names. It was generated from
`src/com/google/javascript/jscomp/CommandLineRunner.java` at
`google/closure-compiler@10ca677aff381d2c2e6e1b254ba32861e503173d`.

The upstream source is not vendored. Obtain it independently, verify that it
is the pinned 87,726-byte file with SHA-256
`153d1bf4d285f3a40ef032acb36b77b9798b87e53da26de212014f68641203c7`,
then regenerate from the `closurec` package directory:

```sh
cargo run --example cli_surface_audit -- \
  --upstream-source /path/to/CommandLineRunner.java \
  --output tests/cli-surface/v20260915-audit.json
```

The generator checks the source pin before parsing any `@Option` declaration.
The `cli_surface` integration test then validates the checked-in report against
the exact current `cli.spec.json` without network access.
