# NN38: Native Rust Dense Framework

Status: native library and local native training/resume implemented; real
repository-workspace CI is required before delivery. Three-language checkpoint
interchange remains a separate pending tranche.

## Scope and ownership

Implement NN37 independently in `code/packages/rust/neural-framework`, using the
existing Rust `matrix` arithmetic foundation. This is native Rust training,
not a wrapper around Python, TypeScript, or an external ML runtime. Keep the
optional C ABI, language bindings, tensor autograd, and new architectures out
of this tranche.

Split delivery into a native implementation tranche and a subsequent required
cross-language conformance tranche. The first must provide a reusable library,
native example, portable checkpoint loading/saving/resume, and local fixtures;
it does not establish three-language interchange until the second actually runs.

## Public API

Expose owned `Activation`, `DenseLayer`, `DenseNetwork`, `GradientTrace`, and
checkpoint records with idiomatic `Result` errors for malformed models, batches,
metadata, and non-finite arithmetic. Model fields remain private. Layer records
and traces may be inspected or copied without granting mutation of model state.

- `DenseNetwork::new(layers)` validates at least one connected dense layer.
- `DenseNetwork::seeded(widths, activations, seed)` implements NN37's wrapping
  unsigned 32-bit LCG and exact input/output draw order.
- `predict`, `gradients`, `train_step`, and `fit` retain NN37's MSE reduction,
  activation derivatives, pre-update trace, exact successful step count, and
  `epochs + 1` loss history, including the zero-epoch case.
- `to_checkpoint` and `from_checkpoint` round-trip owned V1 data. JSON entry
  points reject unknown fields, malformed numeric/activation metadata, and
  incompatible shapes. Export only the interoperable NN37 field set.

The optional JSON-text reader uses the bounded-JSON foundation's 128-level
nesting limit and a 4 MiB byte envelope. The owned checkpoint record API remains
available for larger already-allocated records. Rust's typed `usize` and `u32`
construction parameters replace dynamic-language count/seed type checks; reject
unrepresentable element/byte capacities before allocation.

Step counts must remain within JavaScript's nonnegative safe-integer range,
including overflow checking before an update. Inputs/targets must be nonempty,
rectangular, finite, and dimensionally compatible. Validate raw vectors before
constructing matrix objects; the matrix foundation exposes unchecked public
shape fields and must not become the neural API's validation boundary.

Train into a candidate model. Commit parameters and the incremented step only
after every candidate parameter, prediction, and loss is finite. Failure leaves
the original model and step untouched. ReLU's derivative at zero is zero.

## Numerical and portable evidence

Read `fixtures/neural-framework-v1/dense.json` directly. Assert the paper forward
pass, loss, all weight/bias gradients, one update, seeded weights, and XOR loss
below 0.01 after exactly 3000 updates. Check every parameter by central finite
differences in unequal-width, multiple-output deep networks and all activations.
Cover construction, input, shape, metadata, ownership, rate/count boundaries,
non-finite overflow, step exhaustion, and failed-update atomicity.

The native XOR example writes a JSON receipt and accepts a checkpoint on stdin
with `--resume`, enabling the following conformance tranche to exchange actual
parameters with both existing native lanes. Save/reload and uninterrupted versus
resumed native training must already agree within 1e-9 in this tranche.

Provide package metadata, README, changelog, BUILD fronts, workspace membership,
and tests with a 95% coverage target. Run formatting, tests, Clippy, example,
and checkpoint resume directly. Never count a schema or static roster as native
execution. Before a sparse local build, measure free disk; use a disposable
minimal validation workspace containing exact source inputs if resolving the
entire monorepo workspace would materialize unrelated crates. Report that
boundary honestly; repository CI must still validate the real workspace lane.

## Follow-up conformance tranche

Extend the required reusable workflow and shell-free bounded interchange runner
to execute Rust independently, compare the existing fixtures, and exchange
checkpoints in both directions with Python and TypeScript. Update the registry
for Rust source/workspace inputs and add explicit native-versus-binding receipts.
Only then mark Rust checkpoint interchange complete in the framework roadmap.
