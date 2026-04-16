module JsonLexerSpec (spec) where

import Test.Hspec
import Lexer (Token, canonicalTokenName, tokenValue)
import JsonLexer

spec :: Spec
spec = describe "JsonLexer" $ do
    it "tokenizes JSON punctuation, literals, and EOF" $ do
        let result = tokenizeJson "{\"enabled\":true,\"count\":2}"
        fmap (map canonicalTokenName) result
            `shouldBe` Right ["LBRACE", "STRING", "COLON", "TRUE", "COMMA", "STRING", "COLON", "NUMBER", "RBRACE", "EOF"]

    it "decodes escaped JSON string content" $ do
        let result = tokenizeJson "\"line\\nfeed\""
        fmap firstTokenValue result `shouldBe` Right "line\\nfeed"

    it "rejects malformed numbers" $ do
        case tokenizeJson "-" of
            Left err -> show err `shouldContain` "unexpected character"
            Right _ -> expectationFailure "expected lexer error"

firstTokenValue :: [Token] -> String
firstTokenValue tokens =
    case tokens of
        token : _ -> tokenValue token
        [] -> ""
