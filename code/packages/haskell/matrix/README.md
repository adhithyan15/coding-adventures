# matrix

A pure Haskell rectangular matrix implementation. Matrices are immutable, use
row-major `Double` storage, and reject ragged input, invalid dimensions, shape
mismatches, and out-of-bounds access explicitly.

## API

- `fromRows`, `scalar`, `rowVector`, `empty`, `zeros`, `identity`, and
  `fromDiagonal` construct matrices.
- `toRows`, `rows`, `cols`, `get`, and `set` inspect or immutably update values.
- `add`, `subtract`, `addScalar`, `subtractScalar`, `scale`, `transpose`, and
  `dot` provide arithmetic.
- `sumElements`, `sumRows`, `sumColumns`, `mean`, `minimumElement`,
  `maximumElement`, `argmin`, and `argmax` provide reductions.
- `mapElements`, `squareRoot`, `absolute`, and `power` transform every element.
- `flatten`, `reshape`, `row`, `column`, and `slice` rearrange or extract data.
- `exactEquals`, `close`, and `closeWithin` compare matrices.

Operations that can fail return `Either String`; all successful operations
return new matrices and leave their inputs unchanged.

## Running the tests

```sh
cabal test all
```
