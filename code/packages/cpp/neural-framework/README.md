# Native C++ dense neural framework

An independent, header-only ISO C++17 learner in `ca::neural_framework`. Native
[matrix](../matrix/README.md), [float-math](../float-math/README.md) and
[bounded-json](../bounded-json/README.md) provide dense operations, nonlinear
math and parsing. This is not a C/Rust wrapper, binding or external ML library.
[NN51](../../../specs/NN51-native-cpp-dense-framework.md) defines the contract.
Seven-native checkpoint exchange is a separate, pending delivery.

```cpp
#include <neural_framework.hpp>
namespace nf = ca::neural_framework;
auto net = nf::Network::seeded({2, 4, 1},
    {nf::Activation::tanh, nf::Activation::sigmoid}, 7);
nf::Matrix x({{0,0}, {0,1}, {1,0}, {1,1}}), y({{0}, {1}, {1}, {0}});
auto losses = net.fit(x, y, 1, 3000); // initial loss, then one per update
auto restored = nf::Network::from_json(net.to_json());
```

## Values and failure boundaries

Weights use input-by-output orientation. `gradients` reports full-batch MSE and
every derivative. `train_step` returns the candidate's loss; `fit` returns
epochs + 1 losses. ReLU's derivative at zero is zero. Initialization uses NN37's
unsigned 32-bit LCG stream, not cryptographic randomness.

Constructor inputs, `layers()`, `checkpoint()`, predictions and gradients are
independent copies. Copy assignment is transactional. Explicit copy operations
suppress destructive move: constructing or assigning from an rvalue also copies,
leaving the source valid. JSON document lifetime never limits network lifetime.

One update computes gradients, an independent candidate and its finite loss
before a nonthrowing swap and step increment. Any failure leaves that step's
model untouched, including `std::bad_alloc`. A multi-epoch `fit` is **not** an
all-epochs transaction: successful epochs remain if a later epoch fails. Zero
epochs still validates the batch/rate and returns the initial loss.

Closed `Error::code()` categories identify argument, capacity, shape, nonfinite,
schema or JSON rejection; messages do not echo checkpoint fragments. Allocation
failures use standard `std::bad_alloc`. Caps: 16 layers, width 64, 256 samples,
65536 parameters, 100000 epochs, step <= 9007199254740991. Matrix storage and
cached dimensions are both validated before invoking matrix operations. These
are work/storage envelopes, not an RSS or process-wide allocation bound.

Checkpoint V1 admits exact root/layer fields, lexical integer metadata, finite
rectangular numbers and connected layers. It rejects duplicates, unknown or
NUL-altered names, boolean/null cells and invalid shapes. NN50 additionally caps
JSON bytes/depth/nodes/actual requested parser allocation. Serialization uses
locale-independent `to_chars`; negative floating zero, extremes and subnormals
round-trip without changing the global locale.

## Build and command

From this package cwd, `sh tools/run.sh` uses the shared ISO scheduler and runs
the library tests and actual fresh/resumed commands for every produced binary.
`tools/run.ps1` uses that scheduler's MSVC/clang-cl front. Strict warnings stay
enabled. The command takes no arguments (3000 epochs), `--resume` (one epoch from
bounded JSON stdin), or fixed `--baseline-3001` / `--baseline-3002` arguments for
full-state validation. Extra arguments fail. Checkpoints cannot choose paths,
executables, working directories or arguments. Its closed NN51 receipt includes
the checkpoint, predictions and full loss history, with seven-native exchange
explicitly pending. Verbose fresh-command harness output is retained in `_build`.

CMake exports `ca::neural_framework` via `find_package(ca_neural_framework CONFIG)`.
Installation includes all three transitive public headers in leaf-to-root order.
The independent consumer under `tests/installed` proves prediction, update and
checkpoint reload using only that installed target.

## Validation status

Local shared-front GCC and LLVM Clang each passed 237 checks, all 55 shared
checkpoint cases and 93 actual allocation-failure positions. Both produced
commands passed full fresh/resume/baseline equality through step 3002; native
XOR loss then was `0.00023289821085910149`. GCC gcov measured 97.56% of 246 library
lines, 96.99% branches executed, 78.06% taken at least once (not 100% coverage).
Native command gcov measured 90.91% of 44 lines, 85.54% branches executed,
54.82% taken; Python validator line coverage was 100% of 47 statements. LLVM
ASan passed 236 library checks and the full native command suite. Its separate
variant omits replacement allocation hooks because Windows ASan owns those
symbols; normal GCC/Clang still run all 93 real fault positions. The installed
CMake consumer compiled, linked and ran prediction/update/reload successfully.
UBSan and actual Linux/macOS/MSVC/clang-cl CI remain pending. Local MinGW GCC
is not Linux evidence.
