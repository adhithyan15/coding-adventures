# coding-adventures-dsp-complex

Pure Python complex-number helper for the DSP layer (DSP00 / DSP01 / VER15).

Provides the `ComplexTensor` type: an interleaved `[real, imag]` view of floating-point data where the trailing dimension is size 2.

## Features

- **Storage Layout**: Interleaved `[re_0, im_0, re_1, im_1, ...]` 1D list of floats.
- **Resource Exhaustion Defense (CWE-400)**: Pre-flight bounds check enforcing `MAX_SAMPLES = 16,777,216` before memory allocations.
- **Finite Float Validation (CWE-20)**: Rejects `NaN`, `+Inf`, and `-Inf` fail-closed.
- **Recursion-Free (CWE-674)**: Strictly iterative execution.
- **Zero Regular Expressions (CWE-1333)**: No regex imports or execution.
- **Pure Capabilities**: Declares zero OS capabilities (`[]`).
