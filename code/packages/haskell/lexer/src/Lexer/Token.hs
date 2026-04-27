module Lexer.Token
    ( TokenType(..)
    , Token(..)
    , LexerError(..)
    , tokenPrecededByNewline
    , tokenContextKeyword
    , renderTokenType
    , effectiveTypeName
    , canonicalTokenName
    , makeToken
    , withTypeName
    , withFlags
    , simpleTokenType
    ) where

import Data.Bits ((.|.))

data TokenType
    = TokenName
    | TokenNumber
    | TokenString
    | TokenKeyword
    | TokenPlus
    | TokenMinus
    | TokenStar
    | TokenSlash
    | TokenEquals
    | TokenEqualsEquals
    | TokenLParen
    | TokenRParen
    | TokenComma
    | TokenColon
    | TokenSemicolon
    | TokenLBrace
    | TokenRBrace
    | TokenLBracket
    | TokenRBracket
    | TokenDot
    | TokenBang
    | TokenNewline
    | TokenIndent
    | TokenDedent
    | TokenEof
    deriving (Eq, Ord, Enum, Bounded)

instance Show TokenType where
    show = renderTokenType

data Token = Token
    { tokenType :: TokenType
    , tokenValue :: String
    , tokenLine :: Int
    , tokenColumn :: Int
    , tokenTypeName :: Maybe String
    , tokenFlags :: Int
    }
    deriving (Eq, Show)

data LexerError = LexerError
    { lexerErrorMessage :: String
    , lexerErrorLine :: Int
    , lexerErrorColumn :: Int
    }
    deriving (Eq)

instance Show LexerError where
    show err =
        lexerErrorMessage err
            ++ " at line "
            ++ show (lexerErrorLine err)
            ++ ", column "
            ++ show (lexerErrorColumn err)

tokenPrecededByNewline :: Int
tokenPrecededByNewline = 1

tokenContextKeyword :: Int
tokenContextKeyword = 2

renderTokenType :: TokenType -> String
renderTokenType tokenTypeValue =
    case tokenTypeValue of
        TokenName -> "Name"
        TokenNumber -> "Number"
        TokenString -> "String"
        TokenKeyword -> "Keyword"
        TokenPlus -> "Plus"
        TokenMinus -> "Minus"
        TokenStar -> "Star"
        TokenSlash -> "Slash"
        TokenEquals -> "Equals"
        TokenEqualsEquals -> "EqualsEquals"
        TokenLParen -> "LParen"
        TokenRParen -> "RParen"
        TokenComma -> "Comma"
        TokenColon -> "Colon"
        TokenSemicolon -> "Semicolon"
        TokenLBrace -> "LBrace"
        TokenRBrace -> "RBrace"
        TokenLBracket -> "LBracket"
        TokenRBracket -> "RBracket"
        TokenDot -> "Dot"
        TokenBang -> "Bang"
        TokenNewline -> "Newline"
        TokenIndent -> "Indent"
        TokenDedent -> "Dedent"
        TokenEof -> "EOF"

effectiveTypeName :: Token -> String
effectiveTypeName token =
    case tokenTypeName token of
        Just customName -> customName
        Nothing -> renderTokenType (tokenType token)

canonicalTokenName :: Token -> String
canonicalTokenName token =
    case tokenTypeName token of
        Just customName -> customName
        Nothing ->
            case tokenType token of
                TokenName -> "NAME"
                TokenNumber -> "NUMBER"
                TokenString -> "STRING"
                TokenKeyword -> "KEYWORD"
                TokenPlus -> "PLUS"
                TokenMinus -> "MINUS"
                TokenStar -> "STAR"
                TokenSlash -> "SLASH"
                TokenEquals -> "EQUALS"
                TokenEqualsEquals -> "EQUALS_EQUALS"
                TokenLParen -> "LPAREN"
                TokenRParen -> "RPAREN"
                TokenComma -> "COMMA"
                TokenColon -> "COLON"
                TokenSemicolon -> "SEMICOLON"
                TokenLBrace -> "LBRACE"
                TokenRBrace -> "RBRACE"
                TokenLBracket -> "LBRACKET"
                TokenRBracket -> "RBRACKET"
                TokenDot -> "DOT"
                TokenBang -> "BANG"
                TokenNewline -> "NEWLINE"
                TokenIndent -> "INDENT"
                TokenDedent -> "DEDENT"
                TokenEof -> "EOF"

makeToken :: TokenType -> String -> Int -> Int -> Token
makeToken tokenTypeValue value line column =
    Token
        { tokenType = tokenTypeValue
        , tokenValue = value
        , tokenLine = line
        , tokenColumn = column
        , tokenTypeName = Nothing
        , tokenFlags = 0
        }

withTypeName :: String -> Token -> Token
withTypeName customName token = token {tokenTypeName = Just customName}

withFlags :: Int -> Token -> Token
withFlags flags token = token {tokenFlags = tokenFlags token .|. flags}

simpleTokenType :: Char -> Maybe TokenType
simpleTokenType ch =
    case ch of
        '+' -> Just TokenPlus
        '-' -> Just TokenMinus
        '*' -> Just TokenStar
        '/' -> Just TokenSlash
        '(' -> Just TokenLParen
        ')' -> Just TokenRParen
        ',' -> Just TokenComma
        ':' -> Just TokenColon
        ';' -> Just TokenSemicolon
        '{' -> Just TokenLBrace
        '}' -> Just TokenRBrace
        '[' -> Just TokenLBracket
        ']' -> Just TokenRBracket
        '.' -> Just TokenDot
        '!' -> Just TokenBang
        _ -> Nothing
