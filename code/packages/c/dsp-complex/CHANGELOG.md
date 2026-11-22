# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Pure ISO C17 port of `dsp-complex` with zero `<math.h>` / `-lm` dependency.
- Interleaved `[re, im]` complex tensor type (`dsp_complex_tensor_t`) and scalar
  complex helpers (`dsp_c_make`, `dsp_c_add`, `dsp_c_sub`, `dsp_c_mul`, `dsp_c_conj`,
  `dsp_c_abs`, `dsp_c_arg`).
- Checked constructors (`dsp_complex_tensor_zeros`, `dsp_complex_tensor_from_real`,
  `dsp_complex_tensor_from_real_imag`, `dsp_complex_tensor_from_interleaved`,
  `dsp_complex_tensor_clone`) with finiteness validation (`NaN`/`Inf` rejection),
  `DSP_COMPLEX_MAX_SAMPLES` (`2^24`) cap, and checked `size_t` arithmetic.
- Safe bounds-checked indexing (`dsp_complex_tensor_get`, `dsp_complex_tensor_set`),
  projections (`real`, `imag`, `magnitude`, `phase`), and element-wise ops
  (`conjugate`, `scale`, `add`, `sub`, `mul`, `div`).
- Zero-on-free idempotent cleanup (`dsp_complex_tensor_free`, `dsp_complex_free_buffer`).
- Unit, hostile-input CWE, and deterministic SplitMix64 PBT tests via `iso-harness`.
