# sql-optimizer (Haskell)

A pure functional, 5-pass logical query optimizer for the Mini-SQLite Level 1 stack.

## What it does

Takes a `LogicalPlan` from `sql-planner` and applies a sequence of rewrite passes,
returning an `OptimizedPlan` — a richer tree that carries optimization annotations
such as required-column sets and scan row-count hints.

## How it fits in the stack

```
SQL text
   ↓  sql-lexer / sql-parser   (tokenize + parse)
Statement AST
   ↓  sql-planner              (resolve columns, build plan tree)
LogicalPlan
   ↓  sql-optimizer            (this package)
OptimizedPlan
   ↓  sql-executor             (future: evaluate against storage)
Result rows
```

## Public API

```haskell
-- Structural lift (no rewrites)
lift :: LogicalPlan -> OptimizedPlan

-- Default 5-pass pipeline
optimize :: LogicalPlan -> OptimizedPlan

-- Custom pipeline
optimizeWithPasses :: [Pass] -> LogicalPlan -> OptimizedPlan

-- Named pass record
data Pass = Pass { passName :: String, passApply :: OptimizedPlan -> OptimizedPlan }

-- The five default passes (in order)
defaultPasses :: [Pass]
```

## OptimizedPlan

Mirrors `LogicalPlan` (one constructor per node) with three additions:

| Addition | Location | Purpose |
|---|---|---|
| `EmptyResult` | top-level constructor | sentinel for provably-empty sub-plans |
| `optRequiredCols` | `OptScan` field | subset of columns the rest of the plan needs |
| `optScanLimit` | `OptScan` field | early-exit row-count hint from a LIMIT above |

## Optimization Passes

### 1. Constant Folding
Bottom-up fold of `SqlExpr` within the plan:
- Arithmetic on two `Literal` values → folded `Literal`
- `AND`/`OR` short-circuit (`TRUE`/`FALSE` literals)
- `NULL` propagation (any op `NULL` → `NULL`, except short-circuits above)
- `UnaryNot`/`UnaryNeg` on `Literal`
- `IsNull`/`IsNotNull` on `Literal`
- Division/modulo by zero is intentionally **not** folded

### 2. Predicate Pushdown
- Splits `AND` conjuncts into individual filters
- Pushes each filter through `Sort` (always safe)
- Pushes through `Distinct` (always safe)
- Pushes through `Project` (when predicate uses only qualified column references)
- Pushes into the matching side of `INNER`/`CROSS` joins
- Pushes to the preserved side of `LEFT`/`RIGHT` outer joins only
- Stops at `Aggregate`, `Limit`, `FULL OUTER JOIN`

### 3. Projection Pruning
Top-down pass carrying a required-column set:
- At `OptScan`: sets `optRequiredCols` to the sorted list of needed columns
- At `OptProject`: required = column refs from output expressions
- At `OptFilter`/`OptHaving`: adds predicate column refs to required set
- At `OptSort`: adds sort key column refs
- `OutputStar` (wildcard) disables pruning conservatively

### 4. Dead Code Elimination
- `Filter(EmptyResult, _)` → `EmptyResult`
- `Filter(_, FALSE)` → `EmptyResult`
- `Filter(_, NULL)` → `EmptyResult`
- `Filter(child, TRUE)` → `child`
- `Limit(_, Just 0, _)` → `EmptyResult`
- `Project`/`Sort`/`Limit`/`Distinct`/`Having`(`EmptyResult`) → `EmptyResult`
- `INNER`/`CROSS JOIN (EmptyResult, _)` or `(_, EmptyResult)` → `EmptyResult`
- `Union(EmptyResult, x)` → `x`; `Union(x, EmptyResult)` → `x`
- `Aggregate(EmptyResult)` → **NOT** collapsed (`COUNT(*)` on empty = 1 row)

### 5. Limit Pushdown
- When `OptLimit(child, Just n, Nothing or Just 0)`: pushes `n` down through `OptProject`/`OptFilter`
- At `OptScan`: sets `optScanLimit = min(existing, n)`
- Stops at `OptSort`, `OptAggregate`, `OptJoin`, `OptDistinct`

## Usage

```haskell
import SqlPlanner
import SqlOptimizer

let schema = inMemorySchema [("users", ["id", "name", "age"])]
let stmt   = SelectStmt { stmtDistinct = False
                        , stmtColumns  = [OutputStar]
                        , stmtFrom     = [TableRef "users" Nothing]
                        , stmtJoins    = []
                        , stmtWhere    = Just (BinaryOp BinGt (Column Nothing "age") (Literal (Just (LitInt 18))))
                        , stmtGroupBy  = []
                        , stmtHaving   = Nothing
                        , stmtOrderBy  = []
                        , stmtLimit    = Just (LimitClause (Just 10) Nothing)
                        }
case plan schema stmt of
    Left err -> print err
    Right lp -> print (optimize lp)
-- OptProject
--   (OptLimit
--     (OptFilter
--       (OptScan "users" Nothing (Just ["age","id","name"]) (Just 10))
--       (BinaryOp BinGt (Column (Just "users") "age") (Literal (Just (LitInt 18)))))
--     (Just 10) Nothing)
--   [OutputStar]
```

## Testing

```bash
cabal test all
```

47 hspec tests covering all five passes plus lift and end-to-end pipelines.
