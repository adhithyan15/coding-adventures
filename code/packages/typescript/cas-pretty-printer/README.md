# @coding-adventures/cas-pretty-printer

Dialect-aware pretty-printer for TypeScript symbolic IR.

The package mirrors the Python/Rust `cas-pretty-printer` layer for the browser
runtime. It supports Lisp prefix output and source-like MACSYMA, Mathematica,
and Maple dialects.

`pretty(expr)` keeps the existing linear output. Use
`pretty(expr, MacsymaDialect, { style: "2d" })` or `pretty2D(expr)` for the
box-based two-dimensional layout used by fractions, powers, square roots,
arithmetic, and lists.
