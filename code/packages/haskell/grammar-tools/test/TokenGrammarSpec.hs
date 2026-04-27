module TokenGrammarSpec (spec) where

import Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import Test.Hspec

import GrammarTools.TokenGrammar

spec :: Spec
spec = do
    describe "parseTokenGrammar" $ do
        it "parses regex and literal definitions in order" $ do
            let grammar =
                    parseTokenGrammar
                        "NUMBER = /[0-9]+/\nPLUS = \"+\"\n"
            fmap (map tokenDefinitionName . tokenGrammarDefinitions) grammar
                `shouldBe` Right ["NUMBER", "PLUS"]

        it "parses keywords, directives, aliases, and groups" $ do
            let source =
                    unlines
                        [ "# @version 3"
                        , "# @case_insensitive true"
                        , "mode: indentation"
                        , "escapes: none"
                        , "NAME = /[a-z]+/"
                        , "STRING_DQ = /\"[^\"]*\"/ -> STRING"
                        , "keywords:"
                        , "  if"
                        , "skip:"
                        , "  WS = /[ \\t]+/"
                        , "group tag:"
                        , "  ATTR = /[a-z]+/"
                        ]
                expectedGroup = PatternGroup "tag" [TokenDefinition "ATTR" "[a-z]+" True 12 Nothing]
            fmap tokenGrammarVersion (parseTokenGrammar source) `shouldBe` Right 3
            fmap tokenGrammarCaseInsensitive (parseTokenGrammar source) `shouldBe` Right True
            fmap tokenGrammarKeywords (parseTokenGrammar source) `shouldBe` Right ["if"]
            fmap tokenGrammarMode (parseTokenGrammar source) `shouldBe` Right (Just "indentation")
            fmap tokenGrammarEscapeMode (parseTokenGrammar source) `shouldBe` Right (Just "none")
            fmap tokenGrammarSkipDefinitions (parseTokenGrammar source)
                `shouldBe` Right [TokenDefinition "WS" "[ \\t]+" True 10 Nothing]
            fmap tokenGrammarGroups (parseTokenGrammar source)
                `shouldBe` Right (Map.fromList [("tag", expectedGroup)])

        it "returns an error for malformed definitions" $ do
            case parseTokenGrammar "NUMBER /[0-9]+/" of
                Left err -> tokenGrammarErrorLineNumber err `shouldBe` 1
                Right _ -> expectationFailure "expected parse error"

    describe "validateTokenGrammar" $ do
        it "reports duplicate names and invalid conventions" $ do
            let grammar =
                    TokenGrammar
                        0
                        False
                        [ TokenDefinition "Name" "[a-z]+" True 1 Nothing
                        , TokenDefinition "Name" "[a-z]+" True 2 (Just "string")
                        ]
                        []
                        (Just "weird")
                        []
                        []
                        (Just "strange")
                        []
                        Map.empty
                        True
                        []
                        []
                issues = validateTokenGrammar grammar
            issues `shouldSatisfy` any (isInfixOf "Duplicate token name 'Name'")
            issues `shouldSatisfy` any (isInfixOf "Token name 'Name' should be UPPER_CASE")
            issues `shouldSatisfy` any (isInfixOf "Alias 'string'")
            issues `shouldSatisfy` any (isInfixOf "Unknown lexer mode 'weird'")

        it "collects token names including aliases" $ do
            let grammar =
                    TokenGrammar
                        0
                        False
                        [TokenDefinition "STRING_DQ" "\"[^\"]*\"" True 1 (Just "STRING")]
                        []
                        Nothing
                        []
                        []
                        Nothing
                        []
                        Map.empty
                        True
                        []
                        []
            tokenNames grammar `shouldBe` Set.fromList ["STRING_DQ", "STRING"]
            effectiveTokenNames grammar `shouldBe` Set.fromList ["STRING"]
