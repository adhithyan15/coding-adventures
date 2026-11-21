# music-machine-ffi

`music-machine-ffi` exposes a confined C ABI for `music-machine-core` (`MUS01`, `MUS03`, `MUS04`, and `MUS07`).

## Security Guarantees (Family-1 Memory Safety)

- **CWE-476 (NULL Pointer Dereference)**: All pointer arguments are validated for NULL, alignment, and pairwise distinctness before dereferencing. Output pointers are initialized to zero/NULL. `mm_free_buffer(NULL, ...)` is guaranteed to be a safe no-op.
- **CWE-125 / CWE-787 (Out-of-Bounds Read/Write)**: Slice construction is bounded strictly by `len <= MAX_SCORE_BYTES = 1,000,000`, checks pointer wraparound, and rejects interior NUL bytes.
- **CWE-190 / CWE-400 (Integer Overflow / Resource Exhaustion)**: Pre-flight checks on `sample_rate_hz` (`[1000, 192000]` when non-zero), `max_samples` (`<= MAX_SAMPLE_COUNT = 16,777,216`), and checked byte-size arithmetic before parsing or allocation.
- **CWE-415 / CWE-416 (Double Free / Use-After-Free)**: Single-owner pointer lifecycle managed via `Box<[i16]>` (`Box::into_raw` and `Box::from_raw`) with volatile zero-on-free wiping in `mm_free_buffer`.
- **Unwind Safety**: Every exported C ABI function wraps its entire body in `std::panic::catch_unwind`, preventing undefined behavior across foreign language boundaries.

## Exported Functions

```c
int32_t mm_render_score_text_to_pcm16(
    const uint8_t *text_ptr,
    size_t text_len,
    uint32_t sample_rate_hz,
    int16_t **out_samples_ptr,
    size_t *out_samples_len,
    size_t *out_clipped_count
);

int32_t mm_render_portable_json_to_pcm16(
    const uint8_t *json_ptr,
    size_t json_len,
    size_t max_samples,
    int16_t **out_samples_ptr,
    size_t *out_samples_len,
    size_t *out_clipped_count
);

int32_t mm_render_portable_text_to_pcm16(
    const uint8_t *text_ptr,
    size_t text_len,
    size_t max_samples,
    int16_t **out_samples_ptr,
    size_t *out_samples_len,
    size_t *out_clipped_count
);

void mm_free_buffer(
    int16_t *ptr,
    size_t len
);
```
