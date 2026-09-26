### Back-compat clause

Every existing package — toolkit, dialog, the ones with one
`.mll` per component — builds byte-for-byte identically.
`discover_variants` returns `[None]` for a component with only a
bare default, the loop runs once, and the artifact filename is
unsuffixed exactly as before. Eight new tests cover this back-compat
path explicitly.

