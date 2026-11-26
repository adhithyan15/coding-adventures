# Unreleased — Experiment plan inputs

Register the two predeclared traversal/growth plans as exact repository inputs
of `go/programs/microbenchmark-experiments`. Plan-only changes select the
program, export both paths to Kiln, and invalidate fallback hashes. Tests cover
both files, language filters, missing consumers, neighboring names and actual
program paths. The paired-results fixture reader set remains separate.
