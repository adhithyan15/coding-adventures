# Six native learners, thirty directions

NN48 proves native C training; NN49 tests a different property: can C continue
each existing learner's checkpoint, and can each learner continue C's checkpoint?
The checkpoint is data, not a language wrapper or an executable program.

For example, `C(3000) -> Ruby(3001) -> C(3002)` means the runner sends C's
weights and biases to Ruby on JSON stdin. Ruby computes a real gradient/update;
C then computes the return update. Both complete receipts must match same-lane
baselines within 1e-9, including every parameter, prediction, loss and exact step.
Native C uses Mat and float-math; it does not call a Rust training backend.

With six languages, each source has five other destinations: `6 * 5 = 30`
directed pairs. Six fresh models, twelve native baseline resumes and two child
executions per pair give `6 + 12 + 2 * 30 = 78` real child executions. Merely
adding C to a roster cannot produce this numerical evidence.

Build C from `code/packages/c/neural-framework` with its existing BUILD front.
The shared ISO harness compiles and validates every discovered compiler's
command; Linux CI requires both GCC and Clang. With the other native tools and
TypeScript dependencies installed, run from the repository root:

```sh
python code/scripts/validate_neural_framework_interchange.py --c-executable code/packages/c/neural-framework/_build/gcc-native-xor
```

On Windows select the actual produced `.exe` path (for example
`_build/clang-cl-native-xor.exe`). The trusted path is build configuration;
checkpoint fields cannot choose argv, cwd or executable. Missing commands,
timeouts, malformed receipts, nonfinite/bool numeric leaves and drift fail.

Local NN49 execution passed all thirty directions through 3002 with zero
binding lanes and XOR loss 0.0002330712337828204. Local Rust used hash-matched
source/test/example copies in a minimal workspace, not the full repository
workspace. Required exact-head CI must independently prove that workspace and
the selected reusable gate before NN49 is marked delivered. Native-only NN48
receipts retain their historical scope; only NN49's real exchange receipt
claims six-language conformance. See the [specification](../../specs/NN49-six-native-dense-conformance.md)
and [required gate contract](../../specs/neural-framework-ci-gate.md).
