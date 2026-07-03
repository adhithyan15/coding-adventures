# nib-type-checker

Rust type checker for Nib.

The checker tracks function-local `let`/parameter bindings plus module-scoped
`static` declarations. Static names are visible from every function, can be read
in expressions, and can be assigned with the declared type as context; a local
binding or parameter of the same name shadows the static.
