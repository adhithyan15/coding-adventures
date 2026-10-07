# Perl IIR compiler (Rust)

Compiles LANG81's bounded Perl 5.38 `print` arithmetic subset directly from
the Rust Perl parser tree to `interpreter-ir`. The `plvm` binary runs it on
Rust `vm-core`. It does not use Semantic IR or a host Perl process at runtime.
