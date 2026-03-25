# Changelog

## [0.1.0] - 2026-03-25

### Added

- Initial implementation of `@coding-adventures/commonmark-native` Node.js addon
- `markdownToHtml(markdown: string): string` — full CommonMark 0.31.2 pipeline
  with raw HTML passthrough for trusted author content
- `markdownToHtmlSafe(markdown: string): string` — safe variant that strips all
  raw HTML blocks and inline HTML to prevent XSS attacks in web applications
- Zero-dependency implementation via `node-bridge` N-API FFI (no napi-rs, no napi-sys)
- Uses `napi_create_function` to expose standalone exported functions (not a class)
- Cross-platform support: Linux (`.so`), macOS (`.dylib`), Windows (`.dll`)
- ABI-stable N-API v1 interface works with any Node.js 8.0.0+
- TypeScript type definitions in `index.d.ts` with full JSDoc documentation
- ESM-compatible `index.js` loader using `createRequire` trick for `.node` files
- Comprehensive Vitest test suite with v8 coverage, testing all CommonMark
  block elements, inline elements, lists, code blocks, raw HTML passthrough,
  safe mode XSS prevention, and error handling
