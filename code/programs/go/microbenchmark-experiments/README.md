# microbenchmark-experiments

This Go reference program turns a predeclared plan into paired measurements and
portable raw reports. It uses `microbenchmark` and `rank-statistics` directly;
Warrant is optional. The reusable libraries have separate language ports, while
this program provides a concrete experiment to study and reproduce.

From this directory, build a binary and choose a new output directory:

```bash
go build -o microbenchmark-experiments .
git rev-parse HEAD
./microbenchmark-experiments --plan ../../../benchmarks/microbenchmarks/smoke-plan.json --out /tmp/bench-smoke --revision FULL_COMMIT_FROM_ABOVE
```

On Windows use the `.exe` suffix and a suitable absolute output path. The small
smoke plan uses forty pairs and the library defaults of 100,000 ns target and
65,536 maximum batch. The larger learning plan predeclares six memory sizes,
six growth lengths and two seeds, with a 1 ms target and larger batch ceiling.
Calibration exhaustion or invalid timing fails instead of silently changing it.

`manifest.json` is published last, after its temporary file is written and closed.
Publishing uses an atomic no-clobber hard link; the output filesystem must support
hard links. Unsupported filesystems fail before creating the completion marker.
The manifest identifies the plan and executable by SHA-256,
retains runtime/build information, timestamps, OS, architecture and logical CPU
count, and lists every report's relative filename and digest. The supplied source
revision is a reproducer assertion; it is not inferred from a binary. Build from
a clean committed source and retain the binary digest. Per-case seeds override
the template config seed for both layout construction and the balanced schedule.

Each case report contains only the portable core schema and standard metadata;
working-set sizes, variant labels and software counts live in the manifest.
`benchmark-tool paired-replay PATH_TO_CASE.json` validates the observations and
recomputes the exact median interval. A failed run can retain partial case files,
but has no successful completion manifest. Existing output directories are refused.

The experiment guide in `code/benchmarks/microbenchmarks/README.md` explains the
three comparisons, which work is timed and the limits of the conclusions. Neither
this program nor its correctness tests require one variant to be faster. Tests
cover malformed plans, data/cycle invariants, independently derived growth counts,
mutable values, fake-clock collection, digest/replay, bounds and failed output.
