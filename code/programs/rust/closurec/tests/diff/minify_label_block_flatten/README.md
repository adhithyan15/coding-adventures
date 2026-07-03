# minify_label_block_flatten — gap-067: single-statement labeled block flattens

Input: `label:{break label}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `label:break label;`

Captured by CLOC14.34 byte-identity exploration.
