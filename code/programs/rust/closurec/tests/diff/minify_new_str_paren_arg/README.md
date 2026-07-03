# minify_new_str_paren_arg — gap-064: string ")" arg misread as empty-paren close

Input: `var z=new A(")");`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var z=new A(")");`

Captured by CLOC14.32 byte-identity exploration.
