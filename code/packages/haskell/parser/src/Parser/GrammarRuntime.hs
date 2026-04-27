module Parser.GrammarRuntime
    ( parseWithGrammar
    ) where

import Data.List (find)
import GrammarTools.ParserGrammar
import Lexer.Token
import Parser.AST
import Parser.RecursiveDescent (ParseError(..))

data ParserState = ParserState
    { parserTokens :: [Token]
    , parserPosition :: Int
    }
    deriving (Eq, Show)

parseWithGrammar :: ParserGrammar -> [Token] -> Either ParseError ASTNode
parseWithGrammar grammar rawTokens =
    case parserGrammarRules grammar of
        [] -> Left (makeParseError "parser grammar has no rules" fallbackToken)
        startRule : _ -> do
            (ast, finalState) <- parseRule grammar (grammarRuleName startRule) initialState
            case peekToken finalState of
                token
                    | canonicalTokenName token == "EOF" ->
                        Right ast
                    | otherwise ->
                        Left (makeParseError "expected EOF" token)
  where
    initialState = ParserState (normalizeTokens rawTokens) 0
    fallbackToken =
        case reverse rawTokens of
            token : _ -> token
            [] -> makeToken TokenEof "" 1 1

parseRule :: ParserGrammar -> String -> ParserState -> Either ParseError (ASTNode, ParserState)
parseRule grammar ruleName state =
    case find ((== ruleName) . grammarRuleName) (parserGrammarRules grammar) of
        Nothing ->
            Left (makeParseError ("unknown grammar rule " ++ show ruleName) (peekToken state))
        Just rule -> do
            (children, nextState) <- parseElement grammar (grammarRuleBody rule) state
            Right (RuleNode ruleName children, nextState)

parseElement :: ParserGrammar -> GrammarElement -> ParserState -> Either ParseError ([ASTNode], ParserState)
parseElement grammar element state =
    case element of
        RuleReference name True ->
            case peekToken state of
                token
                    | canonicalTokenName token == name ->
                        let (_, nextState) = advanceToken state
                         in Right ([TokenNode token], nextState)
                    | otherwise ->
                        Left (makeParseError ("expected token " ++ name) token)
        RuleReference name False -> do
            (node, nextState) <- parseRule grammar name state
            Right ([node], nextState)
        Literal value ->
            case peekToken state of
                token
                    | tokenValue token == value ->
                        let (_, nextState) = advanceToken state
                         in Right ([TokenNode token], nextState)
                    | otherwise ->
                        Left (makeParseError ("expected literal " ++ show value) token)
        Sequence elements ->
            parseSequence grammar elements state
        Alternation choices ->
            parseAlternation grammar choices state
        Repetition child ->
            parseRepetition grammar child False state
        Optional child ->
            case parseElement grammar child state of
                Right result -> Right result
                Left _ -> Right ([], state)
        Group child ->
            parseElement grammar child state
        PositiveLookahead child ->
            case parseElement grammar child state of
                Right _ -> Right ([], state)
                Left err -> Left err
        NegativeLookahead child ->
            case parseElement grammar child state of
                Right _ -> Left (makeParseError "negative lookahead failed" (peekToken state))
                Left _ -> Right ([], state)
        OneOrMoreRepetition child ->
            parseRepetition grammar child True state
        SeparatedRepetition child separator atLeastOne ->
            parseSeparatedRepetition grammar child separator atLeastOne state

parseSequence :: ParserGrammar -> [GrammarElement] -> ParserState -> Either ParseError ([ASTNode], ParserState)
parseSequence grammar elements state =
    go [] state elements
  where
    go acc currentState [] = Right (acc, currentState)
    go acc currentState (next : rest) = do
        (nodes, nextState) <- parseElement grammar next currentState
        go (acc ++ nodes) nextState rest

parseAlternation :: ParserGrammar -> [GrammarElement] -> ParserState -> Either ParseError ([ASTNode], ParserState)
parseAlternation grammar choices state =
    tryChoices Nothing choices
  where
    tryChoices maybeErr [] =
        case maybeErr of
            Just err -> Left err
            Nothing -> Left (makeParseError "expected one of the grammar alternatives" (peekToken state))
    tryChoices maybeErr (choice : rest) =
        case parseElement grammar choice state of
            Right result -> Right result
            Left err -> tryChoices (pickBetterError maybeErr err) rest

parseRepetition :: ParserGrammar -> GrammarElement -> Bool -> ParserState -> Either ParseError ([ASTNode], ParserState)
parseRepetition grammar child requireOne state = do
    (firstNodes, firstState) <-
        if requireOne
            then parseElement grammar child state
            else Right ([], state)
    continue firstNodes firstState
  where
    continue acc currentState =
        case parseElement grammar child currentState of
            Right (nodes, nextState)
                | parserPosition nextState == parserPosition currentState ->
                    Left (makeParseError "grammar repetition did not consume input" (peekToken currentState))
                | otherwise ->
                    continue (acc ++ nodes) nextState
            Left _ -> Right (acc, currentState)

parseSeparatedRepetition ::
       ParserGrammar
    -> GrammarElement
    -> GrammarElement
    -> Bool
    -> ParserState
    -> Either ParseError ([ASTNode], ParserState)
parseSeparatedRepetition grammar child separator atLeastOne state =
    case parseElement grammar child state of
        Left err
            | atLeastOne -> Left err
            | otherwise -> Right ([], state)
        Right (firstNodes, firstState) ->
            continue firstNodes firstState
  where
    continue acc currentState =
        case parseElement grammar separator currentState of
            Left _ -> Right (acc, currentState)
            Right (separatorNodes, afterSeparator) -> do
                (childNodes, nextState) <- parseElement grammar child afterSeparator
                if parserPosition nextState == parserPosition currentState
                    then Left (makeParseError "separated repetition did not consume input" (peekToken currentState))
                    else continue (acc ++ separatorNodes ++ childNodes) nextState

peekToken :: ParserState -> Token
peekToken state =
    case drop (parserPosition state) (parserTokens state) of
        token : _ -> token
        [] -> makeToken TokenEof "" 1 1

advanceToken :: ParserState -> (Token, ParserState)
advanceToken state =
    let token = peekToken state
     in (token, state {parserPosition = parserPosition state + 1})

normalizeTokens :: [Token] -> [Token]
normalizeTokens [] = [makeToken TokenEof "" 1 1]
normalizeTokens tokens =
    case reverse tokens of
        lastToken : _
            | canonicalTokenName lastToken == "EOF" -> tokens
            | otherwise ->
                tokens
                    ++ [ makeToken
                            TokenEof
                            ""
                            (tokenLine lastToken)
                            (tokenColumn lastToken + max 1 (length (tokenValue lastToken)))
                       ]
        [] -> [makeToken TokenEof "" 1 1]

pickBetterError :: Maybe ParseError -> ParseError -> Maybe ParseError
pickBetterError Nothing err = Just err
pickBetterError (Just existing) candidate
    | tokenLine (parseErrorToken candidate) > tokenLine (parseErrorToken existing) = Just candidate
    | tokenLine (parseErrorToken candidate) == tokenLine (parseErrorToken existing)
        && tokenColumn (parseErrorToken candidate) >= tokenColumn (parseErrorToken existing) =
            Just candidate
    | otherwise = Just existing

makeParseError :: String -> Token -> ParseError
makeParseError message token =
    ParseError
        { parseErrorMessage = message
        , parseErrorToken = token
        }
