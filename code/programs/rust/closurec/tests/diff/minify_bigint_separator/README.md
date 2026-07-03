# minify_bigint_separator

Captured from upstream Google Closure Compiler **v20240317**.
Pins that a BigInt literal with `_` separators (`1_000_000n`)
strips the separators to `1000000n`.

Currently **IGNORED** — closurec retains the `_` separators in
BigInt form. See `gap-048` in `code/specs/CLOC12-gaps.md`.

The gap-040 separator-stripping (for regular numbers) does not
extend to BigInt-suffix forms.
