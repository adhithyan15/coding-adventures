module CrossValidatorSpec (spec) where

import Data.List (isInfixOf)
import qualified Data.Map.Strict as Map
import Test.Hspec

import GrammarTools.CrossValidator
import GrammarTools.ParserGrammar
import GrammarTools.TokenGrammar

spec :: Spec
spec = do
    describe "crossValidate" $ do
        it "accepts matching grammars" $ do
            let tokenGrammar =
                    TokenGrammar
                        0
                        False
                        [ TokenDefinition "NUMBER" "[0-9]+" True 1 Nothing
                        , TokenDefinition "PLUS" "+" False 2 Nothing
                        ]
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
                parserGrammar =
                    ParserGrammar
                        0
                        [ GrammarRule "expression" (Sequence [RuleReference "NUMBER" True, RuleReference "PLUS" True, RuleReference "NUMBER" True]) 1
                        ]
            crossValidate tokenGrammar parserGrammar `shouldBe` []

        it "reports missing tokens and unused definitions while respecting aliases and implicit tokens" $ do
            let tokenGrammar =
                    TokenGrammar
                        0
                        False
                        [ TokenDefinition "STRING_DQ" "\"[^\"]*\"" True 1 (Just "STRING")
                        , TokenDefinition "UNUSED" "~" False 2 Nothing
                        ]
                        []
                        (Just "indentation")
                        []
                        []
                        Nothing
                        []
                        Map.empty
                        True
                        []
                        []
                parserGrammar =
                    ParserGrammar
                        0
                        [ GrammarRule
                            "file"
                            (Sequence [RuleReference "STRING" True, RuleReference "INDENT" True, RuleReference "MISSING" True])
                            1
                        ]
                issues = crossValidate tokenGrammar parserGrammar
            issues `shouldSatisfy` any (== "Error: Grammar references token 'MISSING' which is not defined in the tokens file")
            issues `shouldSatisfy` any (isInfixOf "Token 'UNUSED'")
            issues `shouldSatisfy` not . any (isInfixOf "STRING_DQ")
