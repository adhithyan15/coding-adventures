# Elixir Windows BUILD-front fixtures v1

`contract.json` is the language-neutral data contract for the Elixir Windows
toolchain, selected-front audit, and reviewed unsupported packages.
`schema.json` closes its shape.

The fixture contains no executable command supplied to a shell. The command
prefix and suffix describe the exact inert protocol record recognized by the
build tool; package BUILD files supply only stable codes already registered in
the fixture.

Validate the contract and live repository with:

```text
python -m unittest discover -s code/scripts/tests -p 'test_elixir_windows_build_fronts.py'
python code/scripts/validate_elixir_windows_build_fronts.py --format json
```

