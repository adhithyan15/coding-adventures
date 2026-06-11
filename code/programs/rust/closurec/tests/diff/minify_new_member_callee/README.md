# minify_new_member_callee — member-callee new-expr (deferred gap-059 follow-up)

Input: `var x=new a.b.C().d;`

Upstream Closure v20240317 (WHITESPACE_ONLY): `var x=(new a.b.C).d;`

Captured by CLOC14.30 byte-identity exploration.
