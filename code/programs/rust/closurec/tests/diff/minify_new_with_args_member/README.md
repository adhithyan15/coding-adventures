# minify_new_with_args_member — arg-bearing new-expr member (deferred gap-059 follow-up)

Input: `var x=new A(y).b;`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var x=(new A(y)).b;`

Captured by CLOC14.30 byte-identity exploration.
