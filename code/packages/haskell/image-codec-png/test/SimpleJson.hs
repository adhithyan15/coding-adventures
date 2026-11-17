module SimpleJson
    ( Json(..)
    , parseJson
    , lookupField
    , getString
    , getInt
    , getDouble
    , getArray
    , getObject
    , hexToBytes
    , bytesToHex
    , loadCasesJson
    ) where

import Data.Char (digitToInt, intToDigit, isDigit, isSpace)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import System.Directory (doesFileExist)

data Json
    = JObject ![(String, Json)]
    | JArray ![Json]
    | JString !String
    | JNumber !Double
    | JBool !Bool
    | JNull
    deriving (Eq, Show)

lookupField :: String -> [(String, Json)] -> Maybe Json
lookupField = lookup

getString :: Json -> Maybe String
getString (JString s) = Just s
getString _           = Nothing

getInt :: Json -> Maybe Int
getInt (JNumber n) = Just (floor n)
getInt _           = Nothing

getDouble :: Json -> Maybe Double
getDouble (JNumber n) = Just n
getDouble _           = Nothing

getArray :: Json -> Maybe [Json]
getArray (JArray a) = Just a
getArray _          = Nothing

getObject :: Json -> Maybe [(String, Json)]
getObject (JObject o) = Just o
getObject _           = Nothing

parseJson :: String -> Either String Json
parseJson input = do
    (v, rest) <- parseVal (skip input)
    if null (skip rest) then Right v else Left "trailing content after JSON root"

skip :: String -> String
skip = dropWhile isSpace

parseVal :: String -> Either String (Json, String)
parseVal ('{' : r) = parseObject (skip r) []
parseVal ('[' : r) = parseArray (skip r) []
parseVal ('"' : r) = do
    (s, rest) <- parseStr r ""
    Right (JString s, rest)
parseVal ('t' : 'r' : 'u' : 'e' : r) = Right (JBool True, r)
parseVal ('f' : 'a' : 'l' : 's' : 'e' : r) = Right (JBool False, r)
parseVal ('n' : 'u' : 'l' : 'l' : r) = Right (JNull, r)
parseVal s@(c : _)
    | c == '-' || isDigit c = parseNumber s
parseVal s = Left ("unexpected character in JSON: " ++ take 20 s)

parseStr :: String -> String -> Either String (String, String)
parseStr ('"' : r) acc = Right (reverse acc, r)
parseStr ('\\' : c : r) acc =
    let unescaped = case c of
            '"'  -> '"'
            '\\' -> '\\'
            '/'  -> '/'
            'b'  -> '\b'
            'f'  -> '\f'
            'n'  -> '\n'
            'r'  -> '\r'
            't'  -> '\t'
            _    -> c
    in parseStr r (unescaped : acc)
parseStr (c : r) acc = parseStr r (c : acc)
parseStr [] _ = Left "unterminated string literal"

parseNumber :: String -> Either String (Json, String)
parseNumber s =
    let isNumChar ch = isDigit ch || ch == '.' || ch == '-' || ch == '+' || ch == 'e' || ch == 'E'
        (numStr, rest) = span isNumChar s
    in case reads numStr of
        [(n, "")] -> Right (JNumber n, rest)
        _         -> Left ("invalid number literal: " ++ numStr)

parseObject :: String -> [(String, Json)] -> Either String (Json, String)
parseObject ('}' : r) acc = Right (JObject (reverse acc), r)
parseObject ('"' : r) acc = do
    (k, r1) <- parseStr r ""
    case skip r1 of
        ':' : r2 -> do
            (v, r3) <- parseVal (skip r2)
            case skip r3 of
                ',' : r4 -> parseObject (skip r4) ((k, v) : acc)
                '}' : r4 -> Right (JObject (reverse ((k, v) : acc)), r4)
                _        -> Left "expected ',' or '}' in object"
        _ -> Left "expected ':' after key in object"
parseObject _ _ = Left "invalid object syntax"

parseArray :: String -> [Json] -> Either String (Json, String)
parseArray (']' : r) acc = Right (JArray (reverse acc), r)
parseArray s acc = do
    (v, r1) <- parseVal (skip s)
    case skip r1 of
        ',' : r2 -> parseArray (skip r2) (v : acc)
        ']' : r2 -> Right (JArray (reverse (v : acc)), r2)
        _        -> Left "expected ',' or ']' in array"

hexToBytes :: String -> ByteString
hexToBytes = BS.pack . go
  where
    go (a : b : r) = fromIntegral (digitToInt a * 16 + digitToInt b) : go r
    go [] = []
    go [_] = error "odd-length hex string"

bytesToHex :: ByteString -> String
bytesToHex = concatMap byteHex . BS.unpack
  where
    byteHex w = [intToDigit (fromIntegral (w `div` 16)), intToDigit (fromIntegral (w `mod` 16))]

loadCasesJson :: IO Json
loadCasesJson = do
    let candidates =
            [ "../../../specs/fixtures/image-codec-png-v1/cases.json"
            , "code/specs/fixtures/image-codec-png-v1/cases.json"
            , "../../../../code/specs/fixtures/image-codec-png-v1/cases.json"
            ]
    path <- findExisting candidates
    content <- readFile path
    case parseJson content of
        Left err  -> ioError (userError ("cases.json parse error: " ++ err))
        Right val -> return val
  where
    findExisting [] = ioError (userError "could not find image-codec-png cases.json")
    findExisting (p:ps) = do
        exists <- doesFileExist p
        if exists then return p else findExisting ps
