# type-checker-protocol

Pure Haskell implementation of the shared generic type-checker contract used by
compiler frontends in this repository.

## API

- `TypeErrorDiagnostic` records an immutable message and one-based source
  location.
- `TypeCheckResult` carries the fully or partially typed AST and every
  diagnostic collected during the pass; `ok` reports whether the list is empty.
- `TypeChecker` wraps any function from an input AST to a typed result.
- `GenericTypeChecker` supplies reusable phase/kind hook registration,
  exact-before-wildcard dispatch, diagnostic accumulation, and lifecycle reset.
- `HookResult` distinguishes a handled value from deliberate fall-through.

The generic checker is explicitly threaded through hook functions. This keeps
diagnostics and hook registration pure and makes a configured checker safely
reusable across independent runs.

## Example

```haskell
import TypeCheckerProtocol

data Node = Node { nodeKind :: String, nodeText :: String }

type Checker = GenericTypeChecker Node () Node

literalHook :: Hook Node () Node
literalHook node _ checker =
    (Handled node { nodeText = "checked" }, checker)

checker :: Checker
checker =
    registerHook "node" "literal" literalHook $
        defaultGenericTypeChecker nodeKind
```

## Dependencies

The library depends only on `base`. Tests use Hspec.

## Running the tests

```sh
cabal test all
```
