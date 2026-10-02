# Changelog

## Unreleased

### Fixed

- The playground builds again. `src/browser-transpiler.ts` inlined the Lattice
  grammar from `code/grammars/lattice.tokens` and `lattice.grammar`, which moved
  to `code/grammars/lattice/` in #13382; Vite could not resolve the old paths,
  so `vite build` failed and the Pages deploy had not published since. The
  imports now point at `grammars/lattice/`, and `deploy-lattice-docs.yml`
  watches `code/grammars/lattice/**` so a grammar change redeploys the site.
- `BUILD` now runs `npm run build` after the tests, so a broken bundle fails
  the package's own CI instead of first failing the deploy on main.
