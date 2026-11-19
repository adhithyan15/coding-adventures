WD-HARN1c: the `--sabotage always-pass` negative control in `BUILD` and `BUILD_windows`
now runs through the shared checker `code/scripts/warrant_negctl.py -- cargo test ...`
instead of a `grep`/`findstr` pipe. The cargo arguments are unchanged. The checker also
requires a non-zero exit, exactly one TAP plan, test points numbered `1..N` and no
`Bail out!`, and prints the captured output when the control fails. Each BUILD file
declares `# needs-toolchain: python`.
