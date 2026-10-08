---
category: Rust
---

# Match boxed AST enum payloads explicitly in generated iterative disposal code

The first AST disposal generator matched only bare enum payload names and omitted four `Box<T>` variants. Exhaustive Rust matches rejected the omissions at compile time. Match boxed payloads explicitly, then keep exhaustive enum matches and field destructures in checked-in code so future taxonomy changes require a disposal decision. The lessons tool category is case-sensitive: use `Rust`, as listed by its error.
