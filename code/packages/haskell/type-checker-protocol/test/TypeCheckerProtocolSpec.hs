module TypeCheckerProtocolSpec (spec) where

import TypeCheckerProtocol
import Test.Hspec

data SimpleNode = SimpleNode
    { nodeKind :: String
    , nodeValue :: String
    }
    deriving (Eq, Show)

data TypedNode = TypedNode
    { typedKind :: String
    , resolvedType :: String
    }
    deriving (Eq, Show)

type Checker = GenericTypeChecker SimpleNode String SimpleNode

goodChecker :: TypeChecker SimpleNode TypedNode
goodChecker =
    makeTypeChecker $ \node ->
        TypeCheckResult (TypedNode (nodeKind node) "int") []

badChecker :: TypeChecker SimpleNode TypedNode
badChecker =
    makeTypeChecker $ \node ->
        TypeCheckResult
            (TypedNode (nodeKind node) "error")
            [TypeErrorDiagnostic ("Unknown kind: " ++ nodeKind node) 1 1]

baseChecker :: Checker
baseChecker = newGenericTypeChecker nodeKind locate
  where
    locate node
        | nodeKind node == "origin" = (1, 1)
        | otherwise = (7, 9)

literalHook :: Hook SimpleNode String SimpleNode
literalHook node arguments checker =
    (Handled node {nodeValue = "checked" ++ concat arguments}, checker)

brokenHook :: Hook SimpleNode String SimpleNode
brokenHook node _ checker =
    ( Handled node
    , reportError ("bad node: " ++ nodeKind node) node checker
    )

decliningHook :: Hook SimpleNode String SimpleNode
decliningHook _ _ checker = (NotHandled, checker)

wildcardHook :: Hook SimpleNode String SimpleNode
wildcardHook node _ checker =
    (Handled node {nodeValue = "wildcard"}, checker)

ruleChecker :: Checker
ruleChecker =
    registerHook "node" "broken" brokenHook $
        registerHook "node" "literal" literalHook baseChecker

runNode :: SimpleNode -> Checker -> (SimpleNode, Checker)
runNode node checker =
    case dispatch "node" node [] checker of
        (Nothing, finished) -> (node, finished)
        (Just typed, finished) -> (typed, finished)

spec :: Spec
spec = do
    describe "diagnostics and results" $ do
        it "constructs comparable ordered diagnostics" $ do
            let diagnostic = TypeErrorDiagnostic "Type mismatch" 3 7
            diagnostic `shouldBe` TypeErrorDiagnostic "Type mismatch" 3 7
            diagnostic `shouldNotBe` TypeErrorDiagnostic "Type mismatch" 4 7
            show diagnostic `shouldContain` "Type mismatch"
            line diagnostic `shouldBe` 3
            column diagnostic `shouldBe` 7
            diagnostic `shouldSatisfy` (< TypeErrorDiagnostic "Type mismatch" 4 7)

        it "reports success exactly when no diagnostics exist" $ do
            let typed = TypedNode "literal" "int"
            ok (TypeCheckResult typed []) `shouldBe` True
            ok (TypeCheckResult typed [TypeErrorDiagnostic "bad" 1 1])
                `shouldBe` False

        it "preserves partial typed ASTs and diagnostic order" $ do
            let typed = TypedNode "broken" "error"
                first = TypeErrorDiagnostic "first" 1 1
                second = TypeErrorDiagnostic "second" 2 5
                result = TypeCheckResult typed [first, second]
            typedAst result `shouldBe` typed
            errors result `shouldBe` [first, second]
            show result `shouldContain` "broken"
            result `shouldBe` TypeCheckResult typed [first, second]

    describe "type checker protocol" $ do
        it "accepts checker functions with different outcomes" $ do
            let good = check goodChecker (SimpleNode "literal" "42")
                bad = check badChecker (SimpleNode "??" "")
            ok good `shouldBe` True
            typedKind (typedAst good) `shouldBe` "literal"
            resolvedType (typedAst good) `shouldBe` "int"
            ok bad `shouldBe` False
            message (head (errors bad)) `shouldContain` "??"

    describe "generic checker dispatch" $ do
        it "dispatches exact node kinds through registered hooks" $ do
            let result = runGenericCheck runNode ruleChecker (SimpleNode "literal" "before")
            ok result `shouldBe` True
            nodeValue (typedAst result) `shouldBe` "checked"

        it "records errors through the configured source locator" $ do
            let result = runGenericCheck runNode ruleChecker (SimpleNode "broken" "")
            ok result `shouldBe` False
            errors result
                `shouldBe` [TypeErrorDiagnostic "bad node: broken" 7 9]

        it "leaves unhandled node kinds unchanged" $ do
            let original = SimpleNode "unknown" "unchanged"
                result = runGenericCheck runNode ruleChecker original
            typedAst result `shouldBe` original
            ok result `shouldBe` True

        it "tries hooks in registration order until one handles the node" $ do
            let checker =
                    registerHook "node" "literal" literalHook $
                        registerHook "node" "literal" decliningHook baseChecker
                result = runGenericCheck runNode checker (SimpleNode "literal" "")
            nodeValue (typedAst result) `shouldBe` "checked"

        it "tries exact hooks before wildcard hooks" $ do
            let checker =
                    registerHook "node" "literal" literalHook $
                        registerHook "node" "*" wildcardHook baseChecker
                exactResult = runGenericCheck runNode checker (SimpleNode "literal" "")
                wildcardResult = runGenericCheck runNode checker (SimpleNode "other" "")
            nodeValue (typedAst exactResult) `shouldBe` "checked"
            nodeValue (typedAst wildcardResult) `shouldBe` "wildcard"

        it "forwards dispatch arguments to hooks" $ do
            let (result, finished) =
                    dispatch
                        "node"
                        (SimpleNode "literal" "")
                        ["!", "?"]
                        ruleChecker
            result `shouldBe` Just (SimpleNode "literal" "checked!?")
            checkerErrors finished `shouldBe` []

        it "retains diagnostics from hooks that deliberately fall through" $ do
            let reportingDecline node _ current =
                    (NotHandled, reportError "declined" node current)
                configured =
                    registerHook "node" "literal" literalHook $
                        registerHook "node" "literal" reportingDecline baseChecker
                result = runGenericCheck runNode configured (SimpleNode "literal" "")
            nodeValue (typedAst result) `shouldBe` "checked"
            errors result `shouldBe` [TypeErrorDiagnostic "declined" 7 9]

        it "returns no value after the last hook declines" $ do
            let checker = registerHook "node" "literal" decliningHook baseChecker
                (result, finished) =
                    dispatch "node" (SimpleNode "literal" "") [] checker
            result `shouldBe` Nothing
            checkerErrors finished `shouldBe` []

        it "exposes comparable hook outcomes" $ do
            (Handled "value" :: HookResult String) `shouldBe` Handled "value"
            (NotHandled :: HookResult String) `shouldNotBe` Handled "value"
            show (NotHandled :: HookResult String) `shouldBe` "NotHandled"

    describe "diagnostic lifecycle" $ do
        it "uses the default one-based location" $ do
            let checker = defaultGenericTypeChecker nodeKind :: Checker
                reported = reportError "bad" (SimpleNode "x" "") checker
                (unhandled, _) = dispatch "node" (SimpleNode "x" "") [] checker
            checkerErrors reported `shouldBe` [TypeErrorDiagnostic "bad" 1 1]
            unhandled `shouldBe` Nothing

        it "can derive locations from the reported subject" $ do
            let reported = reportError "at origin" (SimpleNode "origin" "") baseChecker
            checkerErrors reported `shouldBe` [TypeErrorDiagnostic "at origin" 1 1]

        it "preserves reporting order and resets cleanly" $ do
            let first = reportError "first" (SimpleNode "x" "") baseChecker
                second = reportError "second" (SimpleNode "y" "") first
            map message (checkerErrors second) `shouldBe` ["first", "second"]
            checkerErrors (resetErrors second) `shouldBe` []

        it "starts each generic check without stale diagnostics" $ do
            let dirty = reportError "stale" (SimpleNode "x" "") ruleChecker
                result = runGenericCheck runNode dirty (SimpleNode "literal" "")
            ok result `shouldBe` True
            errors result `shouldBe` []

    describe "node kind normalization" $ do
        it "collapses punctuation and trims separators" $ do
            normalizeKind "expr:add" `shouldBe` "expr_add"
            normalizeKind "  fn decl " `shouldBe` "fn_decl"
            normalizeKind "a---b___c" `shouldBe` "a_b_c"

        it "keeps ASCII case and digits while collapsing non-ASCII text" $ do
            normalizeKind "Node42" `shouldBe` "Node42"
            normalizeKind "lambda λ expr" `shouldBe` "lambda_expr"
            normalizeKind "***" `shouldBe` ""
