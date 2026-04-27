module JsonSerializer
    ( SerializerConfig(..)
    , renderJson
    , renderPrettyJson
    ) where

import Data.List (intercalate, sortOn)
import Numeric (showHex)
import JsonValue (JsonValue(..))

data SerializerConfig = SerializerConfig
    { indentSize :: Int
    , indentChar :: Char
    , sortKeys :: Bool
    , trailingNewline :: Bool
    }
    deriving (Eq, Show)

renderJson :: JsonValue -> String
renderJson value =
    case value of
        JsonNull -> "null"
        JsonBool True -> "true"
        JsonBool False -> "false"
        JsonNumber numberValue
            | isWholeNumber numberValue -> show (round numberValue :: Integer)
            | otherwise -> show numberValue
        JsonString textValue -> "\"" ++ concatMap escapeJsonChar textValue ++ "\""
        JsonArray values -> "[" ++ intercalate "," (map renderJson values) ++ "]"
        JsonObject fields ->
            "{"
                ++ intercalate
                    ","
                    [ renderJson (JsonString fieldName) ++ ":" ++ renderJson fieldValue
                    | (fieldName, fieldValue) <- fields
                    ]
                ++ "}"

renderPrettyJson :: SerializerConfig -> JsonValue -> String
renderPrettyJson config value =
    renderIndented 0 value ++ if trailingNewline config then "\n" else ""
  where
    renderIndented depth currentValue =
        case currentValue of
            JsonArray [] -> "[]"
            JsonArray values ->
                "[\n"
                    ++ intercalate
                        ",\n"
                        [ indent (depth + 1) ++ renderIndented (depth + 1) inner
                        | inner <- values
                        ]
                    ++ "\n"
                    ++ indent depth
                    ++ "]"
            JsonObject [] -> "{}"
            JsonObject fields ->
                let ordered =
                        if sortKeys config
                            then sortOn fst fields
                            else fields
                 in "{\n"
                        ++ intercalate
                            ",\n"
                            [ indent (depth + 1) ++ renderJson (JsonString keyValue) ++ ": " ++ renderIndented (depth + 1) fieldValue
                            | (keyValue, fieldValue) <- ordered
                            ]
                        ++ "\n"
                        ++ indent depth
                        ++ "}"
            _ -> renderJson currentValue
    indent depth =
        replicate (depth * indentSize config) (indentChar config)

isWholeNumber :: Double -> Bool
isWholeNumber numberValue =
    numberValue == fromIntegral (round numberValue :: Integer)

escapeJsonChar :: Char -> String
escapeJsonChar charValue =
    case charValue of
        '"' -> "\\\""
        '\\' -> "\\\\"
        '\b' -> "\\b"
        '\f' -> "\\f"
        '\n' -> "\\n"
        '\r' -> "\\r"
        '\t' -> "\\t"
        '/' -> "/"
        _
            | fromEnum charValue < 0x20 ->
                "\\u" ++ leftPad4 (showHex (fromEnum charValue) "")
            | otherwise -> [charValue]

leftPad4 :: String -> String
leftPad4 textValue =
    replicate (max 0 (4 - length textValue)) '0' ++ textValue
