# Perl IIR compiler (Rust)

Compiles LANG81's bounded Perl 5.38 `print` arithmetic subset directly from
the Rust Perl parser tree to `interpreter-ir`. The `plvm` binary runs it on
Rust `vm-core`. It does not use Semantic IR or a host Perl process at runtime.
The pilot accepts plain decimal integer literals within signed 32-bit range;
legacy leading-zero octal spellings are rejected. Direct AST input is bounded
by item count, depth, token and rule-name text, aggregate text, and module
name size before lowering.

Run the bounded example with `cargo run --manifest-path
code/packages/rust/Cargo.toml -p perl-iir-compiler --bin plvm --
code/packages/rust/perl-iir-compiler/examples/arithmetic.pl` from the repository root.
It prints `3-46` without a trailing newline, matching Perl 5.38 for this input.
