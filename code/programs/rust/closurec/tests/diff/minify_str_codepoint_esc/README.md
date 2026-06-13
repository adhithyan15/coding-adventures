# minify_str_codepoint_esc

CLOC14.40 byte-identity fixture (captured from the upstream
Closure JAR v20240317, WHITESPACE_ONLY).

- **Input:** `var s="\u{1F600}";`
- **Note:** gap-090 (CORRECTNESS): closurec drops the backslash of \u{...}/\x/\0 escapes, MANGLING the string value
