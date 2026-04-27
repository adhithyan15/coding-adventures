module CompilerSpec (spec) where

import qualified Data.Map.Strict as Map
import Test.Hspec

import GrammarTools.Compiler
import GrammarTools.ParserGrammar
import GrammarTools.TokenGrammar

spec :: Spec
spec = do
    describe "compileTokenGrammar" $ do
        it "emits a Haskell module with the token grammar payload" $ do
            let grammar =
                    TokenGrammar
                        1
                        False
                        [TokenDefinition "NUMBER" "[0-9]+" True 1 Nothing]
                        ["if"]
                        Nothing
                        []
                        []
                        Nothing
                        []
                        (Map.fromList [("tag", PatternGroup "tag" [TokenDefinition "ATTR" "[a-z]+" True 2 Nothing])])
                        True
                        []
                        []
                code = compileTokenGrammar grammar "example.tokens" "Generated.TokenGrammar"
            code `shouldContain` "DO NOT EDIT"
            code `shouldContain` "example.tokens"
            code `shouldContain` "module Generated.TokenGrammar where"
            code `shouldContain` "tokenGrammarData"
            code `shouldContain` "NUMBER"
            code `shouldContain` "PatternGroup"

    describe "compileParserGrammar" $ do
        it "emits a Haskell module with the parser grammar payload" $ do
            let grammar =
                    ParserGrammar
                        2
                        [GrammarRule "expression" (Alternation [RuleReference "NUMBER" True, RuleReference "NAME" True]) 1]
                code = compileParserGrammar grammar "example.grammar" "Generated.ParserGrammar"
            code `shouldContain` "DO NOT EDIT"
            code `shouldContain` "example.grammar"
            code `shouldContain` "module Generated.ParserGrammar where"
            code `shouldContain` "parserGrammarData"
            code `shouldContain` "Alternation"
