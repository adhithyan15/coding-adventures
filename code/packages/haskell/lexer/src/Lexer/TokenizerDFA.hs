module Lexer.TokenizerDFA
    ( tokenizerDFAStates
    , tokenizerDFAAlphabet
    , classifyChar
    , buildTokenizerDFATransitions
    , newTokenizerDFA
    ) where

import StateMachine.DFA

tokenizerDFAStates :: [String]
tokenizerDFAStates =
    [ "start"
    , "in_number"
    , "in_name"
    , "in_string"
    , "in_operator"
    , "in_equals"
    , "at_newline"
    , "at_whitespace"
    , "done"
    , "error"
    ]

tokenizerDFAAlphabet :: [String]
tokenizerDFAAlphabet =
    [ "digit"
    , "alpha"
    , "underscore"
    , "quote"
    , "newline"
    , "whitespace"
    , "operator"
    , "equals"
    , "open_paren"
    , "close_paren"
    , "comma"
    , "colon"
    , "semicolon"
    , "open_brace"
    , "close_brace"
    , "open_bracket"
    , "close_bracket"
    , "dot"
    , "bang"
    , "eof"
    , "other"
    ]

classifyChar :: Maybe Char -> String
classifyChar maybeChar =
    case maybeChar of
        Nothing -> "eof"
        Just ch
            | ch == ' ' || ch == '\t' || ch == '\r' -> "whitespace"
            | ch == '\n' -> "newline"
            | ch >= '0' && ch <= '9' -> "digit"
            | isAsciiLetter ch -> "alpha"
            | ch == '_' -> "underscore"
            | ch == '"' -> "quote"
            | ch == '=' -> "equals"
            | ch == '+' || ch == '-' || ch == '*' || ch == '/' -> "operator"
            | ch == '(' -> "open_paren"
            | ch == ')' -> "close_paren"
            | ch == ',' -> "comma"
            | ch == ':' -> "colon"
            | ch == ';' -> "semicolon"
            | ch == '{' -> "open_brace"
            | ch == '}' -> "close_brace"
            | ch == '[' -> "open_bracket"
            | ch == ']' -> "close_bracket"
            | ch == '.' -> "dot"
            | ch == '!' -> "bang"
            | otherwise -> "other"

buildTokenizerDFATransitions :: [((String, String), String)]
buildTokenizerDFATransitions =
    startTransitions ++ handlerTransitions ++ selfLoopTransitions "done" ++ selfLoopTransitions "error"
  where
    startTransitions =
        [ (("start", "digit"), "in_number")
        , (("start", "alpha"), "in_name")
        , (("start", "underscore"), "in_name")
        , (("start", "quote"), "in_string")
        , (("start", "newline"), "at_newline")
        , (("start", "whitespace"), "at_whitespace")
        , (("start", "operator"), "in_operator")
        , (("start", "equals"), "in_equals")
        , (("start", "open_paren"), "in_operator")
        , (("start", "close_paren"), "in_operator")
        , (("start", "comma"), "in_operator")
        , (("start", "colon"), "in_operator")
        , (("start", "semicolon"), "in_operator")
        , (("start", "open_brace"), "in_operator")
        , (("start", "close_brace"), "in_operator")
        , (("start", "open_bracket"), "in_operator")
        , (("start", "close_bracket"), "in_operator")
        , (("start", "dot"), "in_operator")
        , (("start", "bang"), "in_operator")
        , (("start", "eof"), "done")
        , (("start", "other"), "error")
        ]
    handlerStates =
        [ "in_number"
        , "in_name"
        , "in_string"
        , "in_operator"
        , "in_equals"
        , "at_newline"
        , "at_whitespace"
        ]
    handlerTransitions =
        [ ((stateName, symbol), "start")
        | stateName <- handlerStates
        , symbol <- tokenizerDFAAlphabet
        ]
    selfLoopTransitions stateName =
        [ ((stateName, symbol), stateName)
        | symbol <- tokenizerDFAAlphabet
        ]

newTokenizerDFA :: Either String DFA
newTokenizerDFA =
    newDFA tokenizerDFAStates tokenizerDFAAlphabet buildTokenizerDFATransitions "start" ["done"]

isAsciiLetter :: Char -> Bool
isAsciiLetter ch =
    (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')
