# Changelog

## 0.1.0 - 2026-09-20

- Add the portable, bounded DER TLV decoder and cursor.
- Consume all 54 language-neutral conformance cases.
- Replace the Windows no-op with real compile and test commands.
- Make each build command select one Perl toolchain atomically so isolated build
  executor shells cannot mix a system interpreter with Strawberry Perl tools.
