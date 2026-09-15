## 0.4.0 — 2026-05-20 (OCT02 phase 4 — Oct end-to-end on LANG VM)

Oct programs now compile end-to-end via `oct-iir-compiler` (OCT02 phase 3,
PR #3878).  Closes the final phase of the OCT02 four-phase plan — every
language in the LANG74 roadmap (Twig, Nib, Brainfuck, Dartmouth BASIC,
Oct) now ships through the shared LANG VM AOT chain.

**Dispatch wiring.**  `compile_source_to_iir`'s `Language::Oct` arm now
calls `oct_iir_compiler::compile_source` and surfaces frontend errors
(`Unsupported8008Intrinsic`, `Type`, `Parse`) through
`LangAotError::FrontendError`.  The `UnsupportedLanguage` arm is no
longer reachable for any built-in `Language` variant — kept in the
enum so adding a new variant remains a one-arm change.

**End-to-end smoke tests** on both Windows + Linux:

- `end_to_end_oct_minimal_main_exits_zero`: `fn main() { let x: u8 = 42; }`
  compiles + links + runs + exits with the synthesised i64-return code 0.
- `end_to_end_oct_user_fn_call_succeeds`: program with `fn double(a: u8) -> u8 { return a + a; }` and `fn main() { let x: u8 = double(21); }` exercises the cross-function `call` reloc.

Verified locally on Windows.

**Lib test updates.**  `oct_returns_clean_unsupported_error` →
`oct_compiles_to_iir`; new `oct_8008_intrinsic_reports_frontend_error`
confirms the rejection path still surfaces a clean error.

