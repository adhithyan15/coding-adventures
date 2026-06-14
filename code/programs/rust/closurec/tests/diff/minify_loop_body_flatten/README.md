# minify_loop_body_flatten — gap-074: loop-body single-statement block flatten

Input: `l:for(;;){continue l}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `l:for(;;)continue l;`

Captured by CLOC14.35 byte-identity exploration.
