# Changelog

## Unreleased

- Parse natural SWI CLP(FD) infix syntax such as `X in 1..4`,
  `[X,Y] ins 1..4`, and `Z #= X + Y`.

## 0.1.0

- Added the first SWI-Prolog parser package backed by `code/grammars/prolog/swi.grammar`
- Added top-level directive collection for `:- ... .` statements
- Added grammar and executable-source support for DCG rules (`-->`)
