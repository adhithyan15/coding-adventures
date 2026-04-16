module LexerSpec (spec) where

import Test.Hspec

import Lexer

spec :: Spec
spec = do
    describe "tokenize" $ do
        it "tokenizes names, operators, numbers, and EOF" $ do
            let tokens = tokenize defaultLexerConfig "x = 1 + 2"
            fmap (map tokenType) tokens `shouldBe`
                Right [TokenName, TokenEquals, TokenNumber, TokenPlus, TokenNumber, TokenEof]

        it "recognizes keywords, strings, and equality operators" $ do
            let config = LexerConfig ["if", "else"]
                tokens = tokenize config "if name == \"ok\""
            fmap (map tokenType) tokens `shouldBe`
                Right [TokenKeyword, TokenName, TokenEqualsEquals, TokenString, TokenEof]
            fmap (\ts -> tokenValue (ts !! 3)) tokens `shouldBe` Right "ok"

        it "marks tokens that follow a newline" $ do
            let tokens = tokenize defaultLexerConfig "x = 1\ny = 2"
            fmap (\ts -> tokenFlags (ts !! 4)) tokens `shouldBe` Right tokenPrecededByNewline

        it "errors on unterminated strings" $ do
            case tokenize defaultLexerConfig "\"oops" of
                Left err -> lexerErrorMessage err `shouldBe` "unterminated string literal"
                Right _ -> expectationFailure "expected lexer error"
