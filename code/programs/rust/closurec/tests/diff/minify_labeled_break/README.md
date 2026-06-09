# minify_labeled_break — labeled break + gap-032 flatten

Input: `loop:while(x){break loop;}`

Upstream Closure v20240317 (WHITESPACE_ONLY): `loop:while(x)break loop;`

Captured by CLOC14.7. Verdict: PASS
