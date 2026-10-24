# Direct Cargo subcommand binaries retain the Cargo command word

When invoking an installed Cargo subcommand binary directly by path, verify its help instead of assuming the `cargo <name>` wrapper's argument shape. `cargo-llvm-cov` expects `cargo-llvm-cov llvm-cov ...`; passing a sibling-looking `report` subcommand at the first position fails before reading coverage data. Use the direct binary's `--help` output to identify whether reporting is a flag on the main subcommand or a separate command.
