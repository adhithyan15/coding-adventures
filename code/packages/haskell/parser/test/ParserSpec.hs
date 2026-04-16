module ParserSpec (spec) where

import Test.Hspec

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

parseSource :: String -> Either ParseError ASTNode
parseSource source =
    case tokenize defaultLexerConfig source of
        Left lexerErrorValue -> error (show lexerErrorValue)
        Right tokens -> parseTokens tokens
