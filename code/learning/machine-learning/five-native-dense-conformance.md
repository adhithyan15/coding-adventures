# Five native learners exchanging one model

This records the delivered NN46 five-language tranche. The current runner is
extended separately by [NN49 six-native conformance](./six-native-dense-conformance.md),
which additionally requires a harness-built native C executable.

NN45 proved Ruby can train a model independently. NN46 asks a different question:
can every native implementation continue every other's model without changing
its meaning? A roster of five language names cannot answer that question.

First each learner trains the same seeded XOR network for 3000 steps. Each then
continues its own checkpoint twice to produce native baselines at 3001/3002.
Next the runner sends each source checkpoint to each different destination,
then sends the destination's result back to the source for one more update.

For example, `Ruby(3000) -> Rust(3001) -> Ruby(3002)` transports actual weights
and biases on JSON stdin. Every parameter, prediction, loss and exact update
count must match the baseline within 1e-9. Ruby's own reverse-mode engine does
the Ruby updates; the Python runner only transports and compares data.

Five fresh runs plus ten native resumes plus forty cross-runtime steps make
55 real child executions covering twenty directed pairs. Receipts explicitly
report five native lanes and no binding lanes. Broken tools, timeouts, malformed
receipts, booleans masquerading as numbers and numerical drift all fail.

The Ruby command uses a discovered trusted executable, fixed source and library
paths and a fixed working directory. Checkpoints cannot choose a command/path.
The existing reusable CI workflow now validates the Ruby matrix leaf and native
framework before exchange. Ruby dependency and validator changes select this
same required gate; selected jobs cannot silently skip or report success from
a static registration. Existing roll-up ownership and other languages remain.

Run `python code/scripts/validate_neural_framework_interchange.py` after native
dependencies are available. Sparse local Rust may use hash-matched minimal
workspace copies with the documented manifest override; that is not full
repository-workspace validation. Required CI must supply the latter evidence.
See [NN46](../../specs/NN46-five-native-dense-conformance.md) and the
[gate contract](../../specs/neural-framework-ci-gate.md).
