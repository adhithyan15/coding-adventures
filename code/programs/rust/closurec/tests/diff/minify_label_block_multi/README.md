# minify_label_block_multi — multi-statement labeled block keeps braces (gap-067 boundary)

Input: `label:{a();break label}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `label:{a();break label};`

Captured by CLOC14.34 byte-identity exploration.
