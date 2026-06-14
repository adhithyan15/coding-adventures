# minify_get_computed_space — gap-073: get/set before computed key needs a space

Input: `var o={get[k](){return 1}};`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var o={get [k](){return 1}};`

Captured by CLOC14.35 byte-identity exploration.
