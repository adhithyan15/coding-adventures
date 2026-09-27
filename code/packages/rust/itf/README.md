# itf (Rust)

Interleaved 2 of 5 encoding with shared 1D barcode layout and backend-neutral
paint scenes. `normalize_itf` follows `barcode-symbologies-v1`; `itf_error_id`
adds stable identifiers while preserving the existing `Result<_, String>` API.
