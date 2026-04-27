# Changelog

## Unreleased

- add the first Rust `jvm-class-file` crate
- parse a conservative JVM class-file subset with safe malformed-length checks
- decode `Code` attributes without recursive nested `Code` parsing
- resolve UTF-8, class, name-and-type, field, method, and loadable constants
- build a minimal one-method class file for tests and small tools
