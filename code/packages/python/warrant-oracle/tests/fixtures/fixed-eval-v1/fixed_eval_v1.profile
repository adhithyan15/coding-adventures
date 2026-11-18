budget 1048576
chunk_size 65536
slim_stride auto
fault op_x_u8 xor-lsb
fault op_x_u8 flip-result-bit
fault op_x_i16 xor-lsb
fault op_x_i32 flip-result-bit
fault op_x_u64 xor-lsb
fault op_x_i64 flip-result-bit
fault op_eq_u64 ignore-high-bit
fault op_eq_u64 invert
fault op_eq_two ignore-high-bit
fault op_bytes2 flip-result-bit
fault op_bytes2 drop-last-result-byte
fault op_bytes2 truncate-last
fault op_bytes flip-result-bit
fault op_bytes drop-last-result-byte
fault op_bytes truncate-last
fault op_r2 flip-result-bit
fault op_r2 drop-last-result-byte
fault op_r flip-result-bit
fault op_r drop-last-result-byte
fault op_r missing-error
fault op_rfix flip-result-bit
