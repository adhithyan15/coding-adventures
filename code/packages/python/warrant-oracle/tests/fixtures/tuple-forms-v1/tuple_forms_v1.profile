budget 16777216
chunk_size 1024
slim_stride auto
fault op_k ok-as-err
fault op_k swap-components
fault op_pair drop-consumed
fault op_dec drop-consumed
fault op_dec missing-error
fault op_zero missing-error
fault op_lit2 swap-components
fault op_lit3 swap-components
fault op_proj_last8 invert
fault op_bytes flip-result-bit
fault op_bytes drop-last-result-byte
fault op_bits flip-result-bit
fault op_bits drop-last-result-byte
fault op_u8 flip-result-bit
omit_example warrant/tuple_forms/example/e_match_ok
omit_example warrant/tuple_forms/example/e_match_err
