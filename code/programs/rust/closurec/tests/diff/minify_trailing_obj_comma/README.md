# minify_trailing_obj_comma

Captured from upstream Google Closure Compiler **v20240317**
under WHITESPACE_ONLY.

Pins that a trailing `,` in an object literal is dropped
(`{a:1,b:2,}` → `{a:1,b:2}`).

closurec matches byte-for-byte. **PASS**.
