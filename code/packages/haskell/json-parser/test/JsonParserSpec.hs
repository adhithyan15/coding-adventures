module JsonParserSpec (spec) where

import Test.Hspec
import Lexer (canonicalTokenName)
import Parser (ASTNode(..), ParseError(..))
import JsonParser

spec :: Spec
spec = describe "JsonParser" $ do
    it "parses nested JSON structures into grammar nodes" $ do
        case tokenizeAndParseJson "{\"user\":{\"name\":\"Ada\"},\"scores\":[1,2]}" of
            Left err -> expectationFailure ("expected successful parse, got " ++ show err)
            Right ast -> do
                rootRuleName ast `shouldBe` Just "value"
                collectRuleNames ast `shouldSatisfy` all (`elem` ["value", "object", "pair", "array"])
                collectRuleNames ast `shouldSatisfy` \names -> all (`elem` names) ["object", "pair", "array", "value"]
                collectTokenNames ast `shouldSatisfy` \names -> all (`elem` names) ["LBRACE", "STRING", "COLON", "LBRACKET", "NUMBER", "RBRACKET", "RBRACE"]

    it "reports parser errors for malformed JSON" $ do
        case tokenizeAndParseJson "{\"name\":}" of
            Left (JsonParserParseError err) -> parseErrorMessage err `shouldContain` "expected token"
            Left other -> expectationFailure ("expected parser error, got " ++ show other)
            Right _ -> expectationFailure "expected parse failure"

rootRuleName :: ASTNode -> Maybe String
rootRuleName ast =
    case ast of
        RuleNode name _ -> Just name
        _ -> Nothing

collectRuleNames :: ASTNode -> [String]
collectRuleNames ast =
    case ast of
        RuleNode name children -> name : concatMap collectRuleNames children
        TokenNode _ -> []
        _ -> []

collectTokenNames :: ASTNode -> [String]
collectTokenNames ast =
    case ast of
        RuleNode _ children -> concatMap collectTokenNames children
        TokenNode token -> [canonicalTokenName token]
        _ -> []
