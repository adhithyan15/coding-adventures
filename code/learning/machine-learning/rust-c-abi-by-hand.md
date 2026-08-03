# A Rust C ABI, by hand

<!-- learning-concepts: neural-learning-capi -->

The NN34 programs agree on an answer, but each program still performs its own
arithmetic. What if many languages should share one fast Rust implementation?

They need a doorway. A **foreign-function interface**, usually shortened to
**FFI**, is a doorway through which one language calls code built by another.
The doorway used here is a **C ABI**. ABI means *application binary interface*:
the exact low-level agreement about function names, argument shapes, return
values, and memory ownership.

“C” describes the agreement, not the caller. Python, Ruby, Go, Swift, C#, and
many other runtimes know how to call a C-shaped function.

## Keep the model small enough for paper

The Rust core starts with the same NN03 neuron:

```text
inputs  = [2, -1]
weights = [0.5, -0.25]
bias    = 0.1
```

Multiply corresponding values:

```text
contribution 0 =  2 *  0.5  = 1.0
contribution 1 = -1 * -0.25 = 0.25
```

Then add the bias:

```text
prediction = 1.0 + 0.25 + 0.1 = 1.35
```

The identity activation changes nothing, so `1.35` is the final answer. The C
function returns both contributions as well as the prediction. That makes the
native call as auditable as the paper calculation.

## Read the function from left to right

The complete declaration is long, but each piece has one job:

```c
uint32_t neural_learning_weighted_sum_f64_v1(
    const double *inputs,
    const double *weights,
    uint64_t input_count,
    double bias,
    double *contributions_out,
    uint64_t contributions_capacity,
    double *prediction_out);
```

- `const double *inputs` points to numbers the core may read but not change.
- `const double *weights` does the same for weights.
- `uint64_t input_count` says how many input-weight pairs are readable.
- `double bias` passes one binary64 value directly.
- `double *contributions_out` points to caller-owned writable slots.
- `uint64_t contributions_capacity` proves how many output slots exist.
- `double *prediction_out` points to one caller-owned writable slot.
- the returned `uint32_t` is a closed status code, not the prediction.

The `_v1` suffix makes evolution visible. A future incompatible function can
use `_v2` without silently changing what an already-compiled caller means.

## Why the caller owns every buffer

Rust and a foreign runtime may use different allocators. If Rust allocated a
vector and Ruby or C tried to free it, the mismatch could corrupt memory.

NN35 avoids the allocator handshake entirely:

1. the caller allocates inputs and outputs;
2. Rust borrows those buffers only for the call;
3. Rust returns a status;
4. the caller eventually frees its own memory in its own normal way.

No Rust `Vec`, `String`, reference, enum layout, or panic crosses the boundary.

## Failure must not leave half an answer

Suppose the caller provides room for only one contribution even though there
are two inputs. The function returns status `3`,
`NEURAL_LEARNING_BUFFER_TOO_SMALL`.

More importantly, it writes neither output. The caller can initialize output
memory to visible sentinels:

```text
contributions_out before = [91, 92]
prediction_out before    = 93
```

After the rejected call:

```text
status                    = 3
contributions_out after  = [91, 92]
prediction_out after     = 93
```

The core follows the same validate-then-write rule for null pointers, zero
inputs, non-finite arithmetic, and overlapping mutable buffers. A foreign
caller never has to guess whether some prefix of an answer is trustworthy.

## Version numbers are data too

`neural_learning_abi_version()` returns `0x00010000`:

```text
high 16 bits = major version = 1
low  16 bits = minor version = 0
```

A binding can check that number before its first compute call. Function names,
the public header, the status messages, and the language-neutral NN35 catalog
all pin the same contract.

## Explore success and failure

Open the **Rust C ABI** view in the ML Learning Visualizer. Select the paper
example or one of five failure probes. The trace shows which check wins, which
status returns, and whether output memory may change.

The browser validates the committed catalog and recomputes its arithmetic. It
does not load the native library. Direct ABI evidence comes from:

```bash
python code/scripts/validate_neural_learning_rust_cabi.py
```

That validator builds the shared library, loads it through Python's C FFI, and
calls the exported symbols. A Rust-only unit test is useful, but it is not a
substitute for crossing the actual ABI.

## What comes next

NN35 defines the shared Rust doorway without replacing the native NN34 lanes.
The next tranche can add binding-backed consumers and record, language by
language, whether a concept is implemented natively, delegated to the Rust
core, or still missing. Both paths must continue to earn the same fixture
receipt and tolerance result.
