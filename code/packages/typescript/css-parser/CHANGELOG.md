# Changelog

## 0.1.1 - 2026-08-17

- Eliminated runtime grammar loading: `parseCSS`/`createCSSParser` now import a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `css.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

## 0.1.0

- Add grammar-driven CSS parser wrapper for the TypeScript package set.
