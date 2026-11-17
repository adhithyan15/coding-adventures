## Unreleased

- **WD-100b security-review follow-up.** The refusal rule is wider: any win32 child component containing `:` (alternate data streams, `CON:`, `nul::$DATA`, `file.txt:NUL`), a reserved *stem* taken before the first dot with trailing spaces trimmed (`CON .txt`), and the added devices `CONIN$`, `CONOUT$` and `COM`/`LPT` plus a superscript 1, 2 or 3. `CLOCK$` and fullwidth `CON` stay ordinary; posix is unchanged. Fixture: 461 cases (containment 141).
- `runner.src_sha256` changed; the `ct-compare` manifest is regenerated.
