module Parser.RecursiveDescent
    ( ParseError(..)
    , parseTokens
    ) where

import Lexer.Token
import Parser.AST

data ParseError = ParseError
    { parseErrorMessage :: String
    , parseErrorToken :: Token
    }
    deriving (Eq, Show)

data ParserState = ParserState
    { parserTokens :: [Token]
    , parserPosition :: Int
    }
    deriving (Eq, Show)

parseTokens :: [Token] -> Either ParseError ASTNode
parseTokens tokens = fst <$> parseProgram initialState
  where
    initialState = ParserState (normalizeTokens tokens) 0

parseProgram :: ParserState -> Either ParseError (ASTNode, ParserState)
parseProgram state = go (skipNewlines state) []
  where
    go currentState statements
        | atEnd currentState = Right (ProgramNode statements, currentState)
        | otherwise = do
            (statementNode, nextState) <- parseStatement currentState
            go (skipNewlines nextState) (statements ++ [statementNode])

parseStatement :: ParserState -> Either ParseError (ASTNode, ParserState)
parseStatement state
    | tokenType (peekToken state) == TokenName
        && tokenType (peekAhead 1 state) == TokenEquals =
            parseAssignment state
    | otherwise =
        parseExpressionStatement state

parseAssignment :: ParserState -> Either ParseError (ASTNode, ParserState)
parseAssignment state = do
    (nameToken, stateAfterName) <- expectToken TokenName state
    (_, stateAfterEquals) <- expectToken TokenEquals stateAfterName
    (valueNode, stateAfterValue) <- parseExpression stateAfterEquals
    stateAfterTerminator <- consumeStatementTerminator stateAfterValue
    Right (AssignmentNode (tokenValue nameToken) valueNode, stateAfterTerminator)

parseExpressionStatement :: ParserState -> Either ParseError (ASTNode, ParserState)
parseExpressionStatement state = do
    (expressionNode, stateAfterExpression) <- parseExpression state
    stateAfterTerminator <- consumeStatementTerminator stateAfterExpression
    Right (ExpressionStmtNode expressionNode, stateAfterTerminator)

parseExpression :: ParserState -> Either ParseError (ASTNode, ParserState)
parseExpression state = do
    (leftNode, stateAfterLeft) <- parseTerm state
    continue leftNode stateAfterLeft
  where
    continue leftNode currentState =
        case matchToken [TokenPlus, TokenMinus] currentState of
            Nothing -> Right (leftNode, currentState)
            Just (operatorToken, stateAfterOperator) -> do
                (rightNode, stateAfterRight) <- parseTerm stateAfterOperator
                continue
                    (BinaryOpNode leftNode (tokenValue operatorToken) rightNode)
                    stateAfterRight

parseTerm :: ParserState -> Either ParseError (ASTNode, ParserState)
parseTerm state = do
    (leftNode, stateAfterLeft) <- parseFactor state
    continue leftNode stateAfterLeft
  where
    continue leftNode currentState =
        case matchToken [TokenStar, TokenSlash] currentState of
            Nothing -> Right (leftNode, currentState)
            Just (operatorToken, stateAfterOperator) -> do
                (rightNode, stateAfterRight) <- parseFactor stateAfterOperator
                continue
                    (BinaryOpNode leftNode (tokenValue operatorToken) rightNode)
                    stateAfterRight

parseFactor :: ParserState -> Either ParseError (ASTNode, ParserState)
parseFactor state =
    case tokenType (peekToken state) of
        TokenNumber ->
            let (token, nextState) = advanceToken state
             in case reads (tokenValue token) of
                    [(value, "")] -> Right (NumberNode value, nextState)
                    _ ->
                        Left
                            ParseError
                                { parseErrorMessage = "invalid numeric literal"
                                , parseErrorToken = token
                                }
        TokenString ->
            let (token, nextState) = advanceToken state
             in Right (StringNode (tokenValue token), nextState)
        TokenName ->
            let (token, nextState) = advanceToken state
             in Right (NameNode (tokenValue token), nextState)
        TokenLParen -> do
            (_, stateAfterOpen) <- expectToken TokenLParen state
            (expressionNode, stateAfterExpression) <- parseExpression stateAfterOpen
            (_, stateAfterClose) <- expectToken TokenRParen stateAfterExpression
            Right (expressionNode, stateAfterClose)
        _ ->
            Left
                ParseError
                    { parseErrorMessage = "unexpected token"
                    , parseErrorToken = peekToken state
                    }

consumeStatementTerminator :: ParserState -> Either ParseError ParserState
consumeStatementTerminator state
    | atEnd state = Right state
    | tokenType (peekToken state) == TokenNewline = Right (skipNewlines state)
    | otherwise =
        Left
            ParseError
                { parseErrorMessage = "expected a newline or EOF"
                , parseErrorToken = peekToken state
                }

skipNewlines :: ParserState -> ParserState
skipNewlines state
    | tokenType (peekToken state) == TokenNewline =
        skipNewlines (snd (advanceToken state))
    | otherwise = state

expectToken :: TokenType -> ParserState -> Either ParseError (Token, ParserState)
expectToken expectedType state
    | tokenType currentToken == expectedType = Right (advanceToken state)
    | otherwise =
        Left
            ParseError
                { parseErrorMessage =
                    "expected "
                        ++ renderTokenType expectedType
                        ++ ", got "
                        ++ renderTokenType (tokenType currentToken)
                , parseErrorToken = currentToken
                }
  where
    currentToken = peekToken state

matchToken :: [TokenType] -> ParserState -> Maybe (Token, ParserState)
matchToken tokenTypes state
    | tokenType (peekToken state) `elem` tokenTypes = Just (advanceToken state)
    | otherwise = Nothing

advanceToken :: ParserState -> (Token, ParserState)
advanceToken state =
    let token = peekToken state
     in (token, state {parserPosition = parserPosition state + 1})

peekToken :: ParserState -> Token
peekToken state =
    case drop (parserPosition state) (parserTokens state) of
        [] ->
            case reverse (parserTokens state) of
                [] -> makeToken TokenEof "" 1 1
                lastToken : _ -> lastToken
        token : _ -> token

peekAhead :: Int -> ParserState -> Token
peekAhead offset state =
    case drop (parserPosition state + offset) (parserTokens state) of
        [] -> peekToken state {parserPosition = length (parserTokens state) - 1}
        token : _ -> token

atEnd :: ParserState -> Bool
atEnd state = tokenType (peekToken state) == TokenEof

normalizeTokens :: [Token] -> [Token]
normalizeTokens [] = [makeToken TokenEof "" 1 1]
normalizeTokens tokens =
    case reverse tokens of
        lastToken : _
            | tokenType lastToken == TokenEof -> tokens
            | otherwise ->
                tokens
                    ++ [ makeToken
                            TokenEof
                            ""
                            (tokenLine lastToken)
                            (tokenColumn lastToken + max 1 (length (tokenValue lastToken)))
                       ]
        [] -> [makeToken TokenEof "" 1 1]
