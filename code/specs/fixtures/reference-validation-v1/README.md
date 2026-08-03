# Reference validation fixture V1

This language-neutral catalog maps every neural-learning spec from NN03 through
NN32 to exactly one fixture root and exactly one executable Python reference
validator. The catalog itself does not claim that a fixture passed: the
orchestrator must execute all 30 validators and propagate any non-zero exit.

Run the complete reference gate from the repository root:

```bash
python code/scripts/validate_reference_fixture_catalog.py
```

The expected roster is 30 fixture families and 33 lab documents. Paths are
repository-relative, normalized POSIX paths. The orchestrator resolves them
under fixed `code/specs` and `code/scripts` roots, invokes Python without a
shell, limits captured output, and applies a per-validator timeout.

Future language consumers should read the same family fixtures directly. A
Rust core or C ABI is an execution option, not a replacement oracle: its output
must still be compared with these committed expectations and tolerances.
