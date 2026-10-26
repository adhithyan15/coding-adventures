# Native C++ dense learning: publish only a finite candidate

NN50 taught how bytes become an owned JSON tree. NN51 adds a separate native
learner above [matrix](../../packages/cpp/matrix/README.md),
[float-math](../../packages/cpp/float-math/README.md) and
[bounded-json](../../packages/cpp/bounded-json/README.md). No C/Rust training
function or external ML library executes the model. A parsed document can die
immediately after loading: the network copies its weights and biases.

## One paper step before a training loop

The shared [dense fixture](../../specs/fixtures/neural-framework-v1/dense.json)
supplies the numbers, rather than repeating them in a second implementation.
For input `[1,2]`, the first linear layer produces `[2.5,0.5]`; the second
produces `2.25`. Against target `1`, squared error is `1.5625`. The output
derivative is `2*(2.25-1) = 2.5`. Passing that derivative backward yields
`dW2=[[6.25],[1.25]]`, `db2=[2.5]`, `dW1=[[2.5,-2.5],[5,-5]]`, and
`db1=[2.5,-2.5]`. Rate `0.01` updates every parameter, predicting `1.77` with
loss `0.5929`. Native tests compare all gradients and every updated parameter.

For multiple samples and outputs the loss is the mean over **both** axes.
Finite-difference tests perturb every parameter in an unequal-width 2→3→2
network, testing linear/sigmoid/tanh/ReLU derivatives. ReLU at zero uses zero.
The seeded unsigned-32 LCG follows NN37's layer/input/output order; zero biases
consume no random draws. This deterministic initializer is not cryptography.

## Ownership is part of the equation

An equation assumes its matrix is rectangular. C++ matrix exposes both nested
storage and cached dimensions, so the network validates both before `dot` or
`transpose`. A caller shortening a row or altering `rows` cannot turn the
dependency's unchecked access into an out-of-bounds read. Constructor inputs,
inspection, predictions, checkpoints and gradients never alias private state.
Rvalue operations copy too, leaving both networks valid independent values.

One SGD step computes gradients, then builds a separate candidate. Finite
parameters alone are not sufficient: candidate predictions or squared loss
could still overflow. Only after evaluating that finite candidate loss does
a nonthrowing swap publish parameters together with the next step. A real
allocation sweep fails every allocation request in turn and checks the entire
serialized state stayed unchanged. Fit is a sequence of atomic steps, not an
atomic epoch block: a later failure preserves earlier successful updates.

## Checkpoint admission, not merely JSON admission

NN50 syntax acceptance cannot establish a valid model. The native loader
enforces exact V1 keys, lexical integer metadata, finite numeric cells,
compatible shapes, activation names and work/storage caps. Its shared corpus
has 55 cases: three accepted, 52 rejected, including six explicit null cases.
Duplicate/unknown/NUL-altered names are rejected. Locale-independent output
preserves signed floating zero, maximum/minimum normals and subnormals.

The fixed XOR command accepts only trusted baseline flags or `--resume` with
bounded stdin JSON. It validates both dataset dimensions before using the
predictions. Each compiler's actual binary is exercised, comparing whole
checkpoints/predictions at steps 3001/3002 against uninterrupted baselines.
Its NN51 receipt explicitly says seven-native exchange is pending: command
equality within C++ is not seven-language interchange.

## Evidence and remaining boundaries

Local Windows GCC/LLVM Clang each passed 237 library checks and 93 real
allocation-failure positions. XOR at step 3000 had loss
`0.0002330712337828205`; at 3002, `0.00023289821085910149`. GCC gcov measured
97.56% of 246 library lines and 90.91% of 44 command lines; the Python command
validator executed 70 of 71 statements (98.59%), including 17 receipt mutations;
only its missed-rejection sentinel was unrun. ASan separately passed 236 library
checks and actual command validation without incompatible global allocation
replacement hooks. Standard runs retain the full sweep. The installed CMake
consumer compiled, linked and ran prediction, update and checkpoint reload.

The production CI planner initially omitted Windows for a native C++ consumer.
NN51 adds a narrowly owned training predicate/actual cl+clang-cl step to the
existing required BUILD roll-up, with executable planner/ownership regressions.
OCaml scheduling/pins remain unchanged; its exact reviewed planner digest is
refreshed with mutation checks. Actual Linux/macOS/MSVC CI, final independent
review, publication and merge remain pending; no local result substitutes for
those. UBSan has not run successfully on this local Windows toolchain.

See the [package](../../packages/cpp/neural-framework/README.md),
[specification](../../specs/NN51-native-cpp-dense-framework.md) and
[delivery roadmap](./FRAMEWORK-ROADMAP.md).
