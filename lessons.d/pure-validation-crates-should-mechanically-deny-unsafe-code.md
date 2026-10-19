# Pure validation crates should mechanically deny unsafe code

## What went wrong

A new dependency-free cryptographic encoding validator contained no unsafe code,
but its initial implementation did not include a crate-level
`#![deny(unsafe_code)]` attribute. That left the documented pure byte-validation
boundary vulnerable to accidental future expansion.

## Better approach

For repository-owned parsing and validation primitives whose capability model
does not require unsafe code, add `#![deny(unsafe_code)]` from the first
implementation. Treat the attribute as part of the boundary, not merely as a
description of the current source.
