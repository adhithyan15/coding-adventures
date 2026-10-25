# Native Ruby neural framework

An independent, CPU-only dense training library. It uses the pure-Ruby
`coding_adventures_matrix` 0.2.x gem (`require "matrix_ml"`), not Ruby's stdlib
Matrix, an optional Rust binding, or an external ML runtime.

```ruby
require "neural_framework"
nf = CodingAdventures::NeuralFramework
model = nf::DenseNetwork.seeded([2, 4, 1], %w[tanh sigmoid], seed: 7)
inputs = [[0, 0], [0, 1], [1, 0], [1, 1]]
targets = [[0], [1], [1], [0]]
history = model.fit(inputs, targets, 3000, 1.0)
restored = nf::DenseNetwork.from_json(model.to_json)
restored.train_step(inputs, targets, 1.0)
```

`DenseLayer.new(weights:, biases:, activation:)` supplies input-by-output
weights. `DenseNetwork.new(layers, step: 0)` copies and validates them.
Activations are `linear`, `sigmoid`, `tanh`, and `relu`; ReLU's derivative at
zero is zero. `predict` returns a batch. `gradients` returns a `GradientTrace`
with MSE loss, predictions, weight gradients and bias gradients. MSE averages
over all samples and output coordinates. The seeded constructor uses NN37's
uint32 LCG and input-major draw order, with zero biases.

`train_step` returns the pre-update trace and commits all candidate parameters
and the incremented read-only `step` only after candidate validation succeeds.
`fit` performs exactly `epochs` successful steps and returns initial plus
post-step losses (one initial loss for zero epochs). Earlier successful steps
remain committed if a later fit step fails. All public arrays are owned copies.

Cells must be finite Integer/Float values; nil, booleans and strings are not
numbers. Counts and metadata require actual integers. Empty/ragged batches,
disconnected layers, overflow, nonpositive rates, unsafe steps beyond 2^53-1
and unrepresentable allocation capacities raise `ArgumentError`.

`to_checkpoint` / `.from_checkpoint` use owned, closed, string-keyed V1 objects.
`to_json` / `.from_json` use JSON, not executable deserialization. Readers reject
unknown/missing fields and impose 4 MiB and 128-level nesting limits. Parsed
document conformance does not claim identical duplicate-key text semantics.
All 55 shared checkpoint cases (3 accepted, 52 rejected) are tested, including
literal-null weight and bias cells.

## Build and native command

From this package, `ruby scripts/build.rb` installs/tests the matrix leaf first,
then runs framework Standard and Rake (tests plus isolated installed-gem smoke).
Windows uses `ruby scripts/build.rb --windows`; only Standard's EndOfLine rule
is excluded there because this repository stores LF files. Both BUILD fronts
also run the shell-free Python command tests and actual Ruby native validator.

For cached local tools: `ruby C:/Ruby34-x64/bin/bundle install --local`, then
`ruby C:/Ruby34-x64/bin/bundle exec rake`. The Gemfile resolves the matrix gem
from `../matrix`; the publishable gem declares its normal 0.2.x dependency.

```sh
ruby -I../matrix/lib -Ilib examples/xor.rb
python ../../../scripts/validate_neural_framework_ruby.py
```

The native example trains XOR for 3000 steps, emits JSON, and accepts only
`--resume` with bounded JSON stdin for one more update. The validator executes
fresh training and two real resumes through step 3002. Command execution uses
fixed argv/cwd and never takes executable paths from checkpoints.

NN45 supplies native Ruby training. NN46 separately integrates Ruby into the
required five-native conformance runner: from repository root, run
`python code/scripts/validate_neural_framework_interchange.py` with Ruby, Go,
Cargo and the TypeScript peer's dependencies available. It executes twenty
directed pairs with full return-trip comparisons through step 3002. A native
save/reload receipt alone does not establish five-native conformance; actual
selected CI must pass. The native-only validator retains its NN45 scope.
See [NN45](../../../specs/NN45-native-ruby-dense-framework.md) and the
[lesson](../../../learning/machine-learning/native-ruby-dense-framework.md).
