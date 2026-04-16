module ParserGrammarSpec (spec) where

import Data.List (isInfixOf)
import qualified Data.Set as Set
import Test.Hspec

import GrammarTools.ParserGrammar

spec :: Spec
spec = do
    describe "parseParserGrammar" $ do
        it "parses sequences, alternation, repetition, optional, and grouping" $ do
            let source =
                    unlines
                        [ "expression = term { ( PLUS | MINUS ) term } ;"
                        , "term = NUMBER | NAME | LPAREN expression RPAREN ;"
                        ]
            fmap (length . parserGrammarRules) (parseParserGrammar source) `shouldBe` Right 2

        it "parses lookahead and separated repetition" $ do
            let source =
                    unlines
                        [ "args = { expression // COMMA }+ ;"
                        , "postfix = primary !NEWLINE PLUS ;"
                        ]
            case parseParserGrammar source of
                Left err -> expectationFailure (show err)
                Right grammar ->
                    tokenReferences grammar `shouldBe` Set.fromList ["COMMA", "NEWLINE", "PLUS"]

    describe "validateParserGrammar" $ do
        it "reports undefined rules, undefined tokens, bad casing, and unreachable rules" $ do
            let grammar =
                    ParserGrammar
                        0
                        [ GrammarRule "Program" (RuleReference "missing" False) 1
                        , GrammarRule "statement" (RuleReference "NUMBER" True) 2
                        , GrammarRule "unused" (Literal "x") 3
                        ]
                issues = validateParserGrammar grammar (Just (Set.fromList ["NAME"]))
            issues `shouldSatisfy` any (isInfixOf "Rule name 'Program' should be lowercase")
            issues `shouldSatisfy` any (== "Undefined rule reference: 'missing'")
            issues `shouldSatisfy` any (== "Undefined token reference: 'NUMBER'")
            issues `shouldSatisfy` any (isInfixOf "Rule 'unused' is defined but never referenced")

        it "collects referenced rule and token names" $ do
            let grammar =
                    ParserGrammar
                        0
                        [ GrammarRule
                            "program"
                            (Sequence [RuleReference "statement" False, RuleReference "EOF" True])
                            1
                        ]
            ruleReferences grammar `shouldBe` Set.fromList ["statement"]
            tokenReferences grammar `shouldBe` Set.fromList ["EOF"]
