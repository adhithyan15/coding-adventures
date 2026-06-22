# Changelog

## 0.2.0 — 2026-06-22 (LANG-FULL E6 layer 1 — field table)

- Add `JvmFieldInfo { access_flags, name, descriptor }` and a `fields:
  Vec<JvmFieldInfo>` table to `JvmClassFile` — previously the class file had no
  field representation (the parser skipped the `fields[]` section). This lets the
  IIR→JVM backend declare module-level **static globals** (LANG-FULL E6): a
  `public static long G_N` per global, accessed via `getstatic`/`putstatic`.
- The parser now reads `field_info` entries (access_flags, name, descriptor;
  attributes skipped) into `fields` instead of discarding them.

## Unreleased

- add the first Rust `jvm-class-file` crate
- parse a conservative JVM class-file subset with safe malformed-length checks
- decode `Code` attributes without recursive nested `Code` parsing
- resolve UTF-8, class, name-and-type, field, method, and loadable constants
- build a minimal one-method class file for tests and small tools
