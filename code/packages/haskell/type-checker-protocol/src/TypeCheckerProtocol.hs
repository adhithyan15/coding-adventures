-- | Shared pure contracts and dispatch helpers for language type checkers.
module TypeCheckerProtocol
    ( TypeErrorDiagnostic (..)
    , TypeCheckResult (..)
    , ok
    , TypeChecker
    , makeTypeChecker
    , check
    , HookResult (..)
    , Hook
    , GenericTypeChecker
    , newGenericTypeChecker
    , defaultGenericTypeChecker
    , registerHook
    , dispatch
    , reportError
    , resetErrors
    , checkerErrors
    , runGenericCheck
    , normalizeKind
    ) where

import Data.List (dropWhileEnd)

-- | A single type error at a one-based source location.
data TypeErrorDiagnostic = TypeErrorDiagnostic
    { message :: String
    , line :: Int
    , column :: Int
    }
    deriving (Eq, Ord, Show)

-- | A fully or partially typed AST together with collected diagnostics.
data TypeCheckResult ast = TypeCheckResult
    { typedAst :: ast
    , errors :: [TypeErrorDiagnostic]
    }
    deriving (Eq, Show)

-- | Whether a type-checking pass completed without diagnostics.
ok :: TypeCheckResult ast -> Bool
ok = null . errors

-- | A structurally compatible checker function.
newtype TypeChecker astIn astOut = TypeChecker
    { check :: astIn -> TypeCheckResult astOut
    }

-- | Wrap a checker function in the shared protocol type.
makeTypeChecker :: (astIn -> TypeCheckResult astOut) -> TypeChecker astIn astOut
makeTypeChecker = TypeChecker

-- | The outcome of a hook invocation.
--
-- 'NotHandled' asks dispatch to continue to the next registered hook. A
-- 'Handled' value stops the search immediately.
data HookResult result
    = NotHandled
    | Handled result
    deriving (Eq, Show)

-- | A pure phase/kind hook. Hooks may return an updated checker so they can
-- report diagnostics while dispatch continues.
type Hook node argument result =
    node ->
    [argument] ->
    GenericTypeChecker node argument result ->
    (HookResult result, GenericTypeChecker node argument result)

-- | Reusable state for node-driven type checkers.
data GenericTypeChecker node argument result = GenericTypeChecker
    { hookBuckets :: [(String, [Hook node argument result])]
    , diagnostics :: [TypeErrorDiagnostic]
    , nodeKindOf :: node -> String
    , locateNode :: node -> (Int, Int)
    }

-- | Construct a checker with explicit node-kind and source-location helpers.
newGenericTypeChecker ::
    (node -> String) ->
    (node -> (Int, Int)) ->
    GenericTypeChecker node argument result
newGenericTypeChecker kindOf locator =
    GenericTypeChecker
        { hookBuckets = []
        , diagnostics = []
        , nodeKindOf = kindOf
        , locateNode = locator
        }

-- | Construct a checker whose diagnostics default to line 1, column 1.
defaultGenericTypeChecker ::
    (node -> String) ->
    GenericTypeChecker node argument result
defaultGenericTypeChecker kindOf = newGenericTypeChecker kindOf (const (1, 1))

-- | Register a hook after existing hooks for the same phase and kind.
--
-- Kinds are normalized in the same way as node kinds. The literal @"*"@ is
-- retained as the wildcard bucket.
registerHook ::
    String ->
    String ->
    Hook node argument result ->
    GenericTypeChecker node argument result ->
    GenericTypeChecker node argument result
registerHook phase kind hook checker =
    checker
        { hookBuckets = insertHook key hook (hookBuckets checker)
        }
  where
    normalizedKind
        | kind == "*" = "*"
        | otherwise = normalizeKind kind
    key = phase ++ ":" ++ normalizedKind

-- | Dispatch to exact-kind hooks first and wildcard hooks second.
--
-- Hooks within a bucket run in registration order. Diagnostics reported by a
-- hook are retained even when that hook returns 'NotHandled'.
dispatch ::
    String ->
    node ->
    [argument] ->
    GenericTypeChecker node argument result ->
    (Maybe result, GenericTypeChecker node argument result)
dispatch phase node arguments checker = dispatchKeys keys checker
  where
    exactKey = phase ++ ":" ++ normalizeKind (nodeKindOf checker node)
    wildcardKey = phase ++ ":*"
    keys = [exactKey, wildcardKey]

    dispatchKeys [] current = (Nothing, current)
    dispatchKeys (key : remainingKeys) current =
        case lookup key (hookBuckets current) of
            Nothing -> dispatchKeys remainingKeys current
            Just hooks ->
                case dispatchHooks hooks current of
                    (Nothing, updated) -> dispatchKeys remainingKeys updated
                    handled -> handled

    dispatchHooks [] current = (Nothing, current)
    dispatchHooks (hook : remainingHooks) current =
        case hook node arguments current of
            (NotHandled, updated) -> dispatchHooks remainingHooks updated
            (Handled result, updated) -> (Just result, updated)

-- | Add a diagnostic using the configured source locator.
reportError ::
    String ->
    node ->
    GenericTypeChecker node argument result ->
    GenericTypeChecker node argument result
reportError diagnosticMessage subject checker =
    checker
        { diagnostics =
            diagnostics checker
                ++ [TypeErrorDiagnostic diagnosticMessage sourceLine sourceColumn]
        }
  where
    (sourceLine, sourceColumn) = locateNode checker subject

-- | Remove diagnostics while preserving registered hooks and helper functions.
resetErrors ::
    GenericTypeChecker node argument result ->
    GenericTypeChecker node argument result
resetErrors checker = checker {diagnostics = []}

-- | Return diagnostics in reporting order.
checkerErrors ::
    GenericTypeChecker node argument result ->
    [TypeErrorDiagnostic]
checkerErrors = diagnostics

-- | Run a concrete traversal with a clean diagnostic lifecycle.
--
-- The traversal returns its typed or partially typed AST and updated checker.
-- Hook configuration remains reusable because all state is immutable.
runGenericCheck ::
    (node -> GenericTypeChecker node argument result -> (typedAst, GenericTypeChecker node argument result)) ->
    GenericTypeChecker node argument result ->
    node ->
    TypeCheckResult typedAst
runGenericCheck run checker ast =
    TypeCheckResult checkedAst (checkerErrors finished)
  where
    (checkedAst, finished) = run ast (resetErrors checker)

-- | Normalize a node kind to ASCII alphanumerics separated by single
-- underscores. Leading and trailing punctuation is discarded.
normalizeKind :: String -> String
normalizeKind = dropWhileEnd (== '_') . dropWhile (== '_') . collapse
  where
    collapse value = reverse (fst (foldl step ([], False) value))

    step (characters, lastWasUnderscore) character
        | isAsciiAlphaNumeric character = (character : characters, False)
        | lastWasUnderscore = (characters, True)
        | otherwise = ('_' : characters, True)

    isAsciiAlphaNumeric character =
        (character >= 'a' && character <= 'z')
            || (character >= 'A' && character <= 'Z')
            || (character >= '0' && character <= '9')

insertHook :: String -> hook -> [(String, [hook])] -> [(String, [hook])]
insertHook key hook [] = [(key, [hook])]
insertHook key hook ((currentKey, hooks) : remaining)
    | key == currentKey = (currentKey, hooks ++ [hook]) : remaining
    | otherwise = (currentKey, hooks) : insertHook key hook remaining
