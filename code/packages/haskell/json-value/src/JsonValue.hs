module JsonValue
    ( JsonValue(..)
    , parseJson
    ) where

import Control.Applicative ((<|>))
import Data.Char (chr, digitToInt, isDigit)
import Text.ParserCombinators.ReadP
    ( ReadP
    , between
    , char
    , choice
    , eof
    , many
    , munch1
    , option
    , pfail
    , readP_to_S
    , satisfy
    , sepBy
    , skipSpaces
    , string
    )

data JsonValue
    = JsonNull
    | JsonBool Bool
    | JsonNumber Double
    | JsonString String
    | JsonArray [JsonValue]
    | JsonObject [(String, JsonValue)]
    deriving (Eq, Show)

parseJson :: String -> Either String JsonValue
parseJson input =
    case [value | (value, rest) <- readP_to_S (skipSpaces *> jsonValueParser <* skipSpaces <* eof) input, null rest] of
        [] -> Left "invalid json"
        values -> Right (last values)

jsonValueParser :: ReadP JsonValue
jsonValueParser =
    skipSpaces *> choice
        [ JsonNull <$ string "null"
        , JsonBool True <$ string "true"
        , JsonBool False <$ string "false"
        , JsonString <$> jsonStringLiteral
        , JsonArray <$> between (char '[' *> skipSpaces) (skipSpaces *> char ']') (jsonValueParser `sepBy` (skipSpaces *> char ',' *> skipSpaces))
        , JsonObject <$> between (char '{' *> skipSpaces) (skipSpaces *> char '}') (jsonPairParser `sepBy` (skipSpaces *> char ',' *> skipSpaces))
        , JsonNumber <$> jsonNumberParser
        ]

jsonPairParser :: ReadP (String, JsonValue)
jsonPairParser = do
    key <- jsonStringLiteral
    skipSpaces
    _ <- char ':'
    skipSpaces
    value <- jsonValueParser
    pure (key, value)

jsonStringLiteral :: ReadP String
jsonStringLiteral =
    between (char '"') (char '"') (many jsonCharacterParser)

jsonCharacterParser :: ReadP Char
jsonCharacterParser =
    escaped <|> plain
  where
    plain = satisfyNot ['"', '\\']
    escaped = do
        _ <- char '\\'
        choice
            [ '"' <$ char '"'
            , '\\' <$ char '\\'
            , '/' <$ char '/'
            , '\b' <$ char 'b'
            , '\f' <$ char 'f'
            , '\n' <$ char 'n'
            , '\r' <$ char 'r'
            , '\t' <$ char 't'
            , unicodeEscape
            ]
    unicodeEscape = do
        _ <- char 'u'
        hexDigits <- countExactly 4 hexDigitParser
        pure (chr (foldl (\acc digitValue -> acc * 16 + digitToInt digitValue) 0 hexDigits))

jsonNumberParser :: ReadP Double
jsonNumberParser = do
    sign <- option "" (string "-")
    whole <- ifZeroPrefixedNumber
    fractional <- option "" ((:) <$> char '.' <*> munch1 isDigit)
    exponentPart <- option "" exponentParser
    pure (read (sign ++ whole ++ fractional ++ exponentPart))
  where
    ifZeroPrefixedNumber =
        string "0"
            <|> ((:) <$> satisfyDigitOneToNine <*> manyDigitParser)
    exponentParser = do
        exponentMarker <- choice [char 'e', char 'E']
        exponentSign <- option "" (string "+" <|> string "-")
        exponentDigits <- munch1 isDigit
        pure (exponentMarker : exponentSign ++ exponentDigits)

manyDigitParser :: ReadP String
manyDigitParser =
    many (satisfy isDigit)

satisfyDigitOneToNine :: ReadP Char
satisfyDigitOneToNine =
    satisfy (`elem` ['1' .. '9'])

hexDigitParser :: ReadP Char
hexDigitParser =
    satisfy (`elem` (['0' .. '9'] ++ ['a' .. 'f'] ++ ['A' .. 'F']))

countExactly :: Int -> ReadP a -> ReadP [a]
countExactly count parser
    | count <= 0 = pure []
    | otherwise = (:) <$> parser <*> countExactly (count - 1) parser

satisfyNot :: [Char] -> ReadP Char
satisfyNot blocked =
    satisfy (`notElem` blocked)
