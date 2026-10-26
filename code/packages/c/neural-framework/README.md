# Native C dense neural framework

Independent ISO C17 dense training above the native `matrix`, `float-math`
and `bounded-json` packages. No libm, foreign runtime, Rust binding, external
JSON parser or ML engine supplies training. Six-native interchange is a
separate pending tranche; this package alone does not prove it.

NN48 is locally implemented and validated. Publication, actual repository CI
and confirmed exact-head merge remain required before delivery.

```c
#include "neural_framework.h"

int main(void) {
    const size_t widths[] = {2, 4, 1};
    const NFActivation activations[] = {NF_TANH, NF_SIGMOID};
    const double x[] = {0, 0, 0, 1, 1, 0, 1, 1}, y[] = {0, 1, 1, 0};
    NFNetwork *network = NULL;
    NFHistory history = {0};
    NFStatus status = nf_seeded(widths, activations, 2, 7, &network);
    if (status == NF_OK) status = nf_fit(network, x, y, 4, 3000, 1, &history);
    nf_history_free(&history);
    nf_free(&network);
    return status == NF_OK ? 0 : 1;
}
```

## API and ownership

`nf_new` copies each `NFLayer`'s row-major input-by-output weights and biases.
`nf_seeded` uses NN37's wrapping uint32 LCG in layer/input/output order; biases
are zero and consume no draws. Initialization is reproducible, not cryptographic.
All four activations are supported: linear, stable sigmoid, tanh and ReLU
(derivative zero at zero). Loss is mean squared error over samples and outputs.

`nf_predict`, `nf_gradients` and `nf_layer` return owned copies, never model
aliases. `nf_train_step` returns the pre-update trace and commits only a finite
candidate with finite batch loss; failure leaves the complete model unchanged.
`nf_fit` returns initial plus each post-update loss, including a single initial
loss for zero epochs. Each update is atomic, not the whole fit call: a later
fit error retains earlier successful updates. Step inspection is read-only.
`nf_clone` is an owned checkpoint snapshot including the step, not an alias.

Initialize network outputs to NULL and aggregate/Mat outputs to zero. Live
outputs are rejected, not overwritten. Release with `nf_free`, `mat_free`,
`nf_trace_free`, `nf_history_free` or `nf_text_free`; these reset handles and
tolerate NULL/repeated release. Public owned trace/matrix records must retain
their owning pointers until release; do not forge pointers or shallow-copy
ownership. Input arrays must be readable for the declared dimensions during
the call and may be freed/modified afterward. Networks are not concurrently
mutable: synchronize training and inspection on the same network externally.

`nf_new_with_memory` injects allocator callbacks/context for network objects,
their clones, histories and text. Context must outlive those outputs. The
unchanged matrix dependency retains its own malloc/free; callbacks do not
intercept its allocations. Seed/schema scratch buffers use ordinary malloc.

## Closed bounds and checkpoints

Bounds are 16 layers, width 64, 256 samples, 65536 total parameters,
100000 epochs per fit, and step 0..9007199254740991. Shape validation checks
all dimensions/connections and element/byte arithmetic before numeric copies
or Mat construction. Models must additionally fit the 65536-node serialized
checkpoint envelope (numeric leaves plus structural nodes). A forward tape is
at most 17*256*64 doubles (~2.2 MiB).
Candidates/traces add bounded allocations; this is a small-model reference,
not a throughput or large-model claim. Nonfinite inputs/arithmetic are errors.

`nf_from_value` consumes NN47 public values and strictly validates closed V1
fields, duplicates/unknown/missing/embedded-NUL names, exact lexical integer
metadata, finite numeric cells, rectangular shapes and connections. Null and
booleans are not numeric zero. It copies before the source document expires.
`nf_from_json` also applies NN47's 4 MiB input, depth 128, 65536-node and
16 MiB arena ceilings; trailing documents and nonfinite numbers fail.

`nf_to_json` emits bounded owned ordinary JSON using 17 significant digits for
double round trips and always a decimal point '.', never changes global locale,
and serializes only parameters/activations/format/version/step. Both text
directions consult the host numeric locale; serialize concurrent process-global
locale changes externally. No checkpoint content selects files, commands or
runtime backends. Duplicate-key semantics intentionally differ from parsers
which discard duplicate evidence before schema loading.

## Validation

`sh tools/run.sh` or `powershell -File tools/run.ps1` invokes the existing shared
ISO harness, not a new compiler scheduler. Linux CI requires GCC and Clang;
macOS supplies Apple Clang; the scoped Windows step requires cl and clang-cl.
Each front runs numerical/schema tests, builds the native command, and executes
its boundary and full baseline/resume validators for every discovered compiler.
Python is build/test tooling only: Unix uses python3 (trusted `PYTHON` override
supported), Windows uses python. The installed C library has no Python runtime
dependency. Strict warnings and extension rejection remain enabled.

Current GCC, Clang, cl and clang-cl each pass 6857 checks: shared paper
prediction/loss/every gradient/update at 1e-12, initialization oracle, active
all-parameter central differences for all four activations in deep unequal-width
multi-output models, ownership/bounds/atomicity, allocation faults, all 55 shared
schema cases (3 accepted/52 rejected including six null cases), and 3000 XOR
updates with loss 0.0002330712337828205, full uninterrupted versus serialized/
resumed models through 3001/3002, strict JSON metadata/duplicate/NUL fields,
locale preservation and bit-exact extreme/subnormal/signed-zero parameters.
Current gcov library coverage is 97.11% of 415 executable lines, 100% branches
executed and 81.76% branches taken. Clang ASan/UBSan and GCC -fanalyzer pass.
A separate CMake find_package consumer compiled/linked against
the installed archive/headers and ran prediction, training and checkpoint resume.
The POSIX front was also run on Windows using its trusted `PYTHON` override;
that is not Linux/macOS execution evidence. Actual CI must supply those results.

The fixed native command accepts only no arguments, `--resume`,
`--baseline-3001` or `--baseline-3002`. Resume reads at most 4 MiB on stdin and
checks loaded model input/output widths against the fixed XOR dataset before
any batch operation. Its real error/shape/fresh/baseline/resume suite also
passes Clang ASan/UBSan. Command gcov is 100% lines (45), 100% branches executed,
93.75% branches taken; the Python shell-free native validator is 95% covered.
Run `python ../../../scripts/validate_neural_framework_c.py --executable PATH`
with a trusted compiled command path to execute five children and compare full
native baseline/resume state through 3002. No checkpoint content controls paths
or argv. The validator explicitly reports six-language interchange pending.

## Static-library installation

CMake 3.20+ builds and installs `coding_adventures::neural_framework`. Its
archive includes unchanged native dependency translation units as separate
members and installs all four public headers under the target's unique include
directory. Consumers need no repository-relative headers or runtime loader.

```sh
cmake -S . -B _build/cmake -DCMAKE_INSTALL_PREFIX="$PWD/_build/install"
cmake --build _build/cmake
cmake --install _build/cmake
cmake -S tests/install -B _build/consumer -DCMAKE_PREFIX_PATH="$PWD/_build/install"
cmake --build _build/consumer
```

The explicit payload includes headers, archive, exported target/config/version,
MIT license, README/changelog/capability manifest and dependency documentation.
Run the resulting `installed_consumer` executable (configuration subdirectory
and `.exe` suffix depend on generator/platform). It trains and resumes a model
using only the installed target. The separate
[teaching trace](../../../learning/machine-learning/native-c-dense-framework.md)
walks one hand-calculable update and explains the ownership boundary.
