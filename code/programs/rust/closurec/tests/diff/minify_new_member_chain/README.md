# minify_new_member_chain — member access on a new-expression

Input: `var x=new A().b;`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var x=(new A).b;`

Captured by CLOC14.29 byte-identity exploration.
