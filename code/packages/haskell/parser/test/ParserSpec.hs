module ParserSpec (spec) where

import Test.Hspec

import GrammarTools (parseParserGrammar)
import Lexer
import Parser

spec :: Spec
spec = do
    describe "parseTokens" $ do
        it "parses assignments and expression statements into a program" $ do
            let result = parseSource "x = 1\ny = x + 2"
            result
                `shouldBe` Right
                    ( ProgramNode
                        [ AssignmentNode "x" (NumberNode 1.0)
                        , AssignmentNode "y" (BinaryOpNode (NameNode "x") "+" (NumberNode 2.0))
                        ]
                    )

        it "respects operator precedence" $ do
            let result = parseSource "1 + 2 * 3"
            result
                `shouldBe` Right
                    ( ProgramNode
                        [ ExpressionStmtNode
                            ( BinaryOpNode
                                (NumberNode 1.0)
                                "+"
                                (BinaryOpNode (NumberNode 2.0) "*" (NumberNode 3.0))
                            )
                        ]
                    )

        it "respects explicit parentheses" $ do
            let result = parseSource "(1 + 2) * 3"
            result
                `shouldBe` Right
                    ( ProgramNode
                        [ ExpressionStmtNode
                            ( BinaryOpNode
                                (BinaryOpNode (NumberNode 1.0) "+" (NumberNode 2.0))
                                "*"
                                (NumberNode 3.0)
                            )
                        ]
                    )

        it "returns a parse error for malformed expressions" $ do
            case parseSource "1 +" of
                Left err -> parseErrorMessage err `shouldBe` "unexpected token"
                Right _ -> expectationFailure "expected parse error"

    describe "parseWithGrammar" $ do
        it "builds a grammar-shaped AST from token references" $ do
            let grammar =
                    either (error . show) id $
                        parseParserGrammar
                            (unlines
                                [ "value = LBRACE pair RBRACE ;"
                                , "pair = STRING COLON NUMBER ;"
                                ]
                            )
                tokens =
                    [ makeToken TokenLBrace "{" 1 1
                    , makeToken TokenString "answer" 1 2
                    , makeToken TokenColon ":" 1 10
                    , makeToken TokenNumber "42" 1 11
                    , makeToken TokenRBrace "}" 1 13
                    ]
            fmap collectRuleNames (parseWithGrammar grammar tokens)
                `shouldBe` Right ["value", "pair"]

parseSource :: String -> Either ParseError ASTNode
parseSource source =
    case tokenize defaultLexerConfig source of
        Left lexerErrorValue -> error (show lexerErrorValue)
        Right tokens -> parseTokens tokens

collectRuleNames :: ASTNode -> [String]
collectRuleNames ast =
    case ast of
        RuleNode name children -> name : concatMap collectRuleNames children
        TokenNode _ -> []
        _ -> []
