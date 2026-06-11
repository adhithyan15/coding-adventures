# minify_new_str_paren_member — gap-064: same bug under arg-bearing member wrap

Input: `var z=new A(")").b;`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var z=(new A(")")).b;`

Captured by CLOC14.32 byte-identity exploration.
