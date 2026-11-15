# @coding-adventures/dsp-complex

Pure, bounded complex number and array representations for the DSP layer.

## Features

- **Complex numbers**: Immutable `Complex` objects with arithmetic (`add`, `sub`, `mul`, `div`), polar coordinates (`fromPolar`, `abs`, `arg`), exponential (`exp`), scaling, and conjugate.
- **Complex arrays**: Contiguous interleaved `Float64Array` `ComplexArray` (aliased as `ComplexTensor`) optimized for high-performance signal processing.
- **VER15 Security Hardening**:
  - Pre-flight bounds checking (`MAX_SAMPLES = 16,777,216`) enforcing CWE-400 memory limits before any buffer allocation.
  - Fail-closed finite float validation (`Number.isFinite`, rejecting `NaN`, `+Infinity`, `-Infinity`, CWE-20).
  - Strictly iterative execution with zero recursion (CWE-674).
  - Pure capabilities: zero external dependencies, no file/network/process access.
