# Twig guest microbenchmark

This Rust library prepares two small Twig programs in the real `twig-vm`, then
times repeated `dispatch::run` calls with the shared VER07 host collector. One
program sums 1 through 32 with ordinary recursion; the other uses an
accumulator. Both run inside the guest interpreter and return the same
checksum. Compilation and warm verification happen before timing.

```powershell
cargo run --locked -p coding_adventures_twig_guest_microbenchmark --bin twig-guest-microbenchmark -- --pairs 40 --revision local --manifest twig-envelope.json > twig-report.json
cargo run --locked -p coding_adventures_twig_guest_microbenchmark --bin twig-guest-microbenchmark -- --replay twig-report.json
cargo run --locked -p coding_adventures_twig_guest_microbenchmark --bin twig-guest-microbenchmark -- --replay-envelope twig-envelope.json
cargo test --locked -p coding_adventures_twig_guest_microbenchmark
```

The CLI writes the standard raw-pair `Report` JSON to stdout and a separate
strict guest envelope to the explicit `--manifest` path. The envelope contains
the exact report, guest engine and call boundary, a host clock/no-op probe, and
21 raw empty-entry sample durations with their calibration settings. These
observations are never subtracted from the report. The measurements are host
wall time for this interpreter workload; they are not simulated CPU cycles.

The shared Rust support library supplies collection and statistics. This is a
guest execution lane, rather than another independent statistics implementation.
Tests replay all portable cases from the package-local fixture. In the repository
they also compare its bytes with the canonical fixture; extracted package source
can run those tests without a sibling microbenchmark package's fixture file.
