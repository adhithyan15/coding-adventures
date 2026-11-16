-- | Minimal zero-dependency JSON parser for the canonical-cbor test suite.
-- Reads code/specs/fixtures/canonical-cbor-v1/cases.json using only GHC boot packages.
module SimpleJson
  ( Json(..)
  , TestCase(..)
  , parseJson
  , loadCorpus
  , hexToBytes
  , bytesToHex
  ) where

import Data.Char (digitToInt, intToDigit, isDigit, isSpace)
import qualified Data.ByteString as BS
import System.Directory (doesFileExist)

data Json
  = JObject ![(String, Json)]
  | JArray ![Json]
  | JString !String
  | JBool !Bool
  | JInt !Integer
  deriving (Eq, Show)

data TestCase = TestCase
  { testId        :: !String
  , testOperation :: !String
  , testInput     :: !String
  , testExpected  :: !String
  } deriving (Eq, Show)

parseJson :: String -> Either String Json
parseJson input = do
  (v, rest) <- parseVal (skip input)
  if null (skip rest) then Right v else Left "trailing content"

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
parseVal s@(c : _)
  | isDigit c =
      let (ds, rest) = span isDigit s
      in Right (JInt (read ds), rest)
parseVal s = Left ("unexpected JSON: " ++ take 20 s)

parseStr :: String -> String -> Either String (String, String)
parseStr ('"' : r) acc = Right (reverse acc, r)
parseStr ('\\' : c : r) acc = parseStr r (c : acc)
parseStr (c : r) acc = parseStr r (c : acc)
parseStr [] _ = Left "unterminated string"

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
        _        -> Left "expected , or } in object"
    _ -> Left "expected : in object"
parseObject _ _ = Left "bad object"

parseArray :: String -> [Json] -> Either String (Json, String)
parseArray (']' : r) acc = Right (JArray (reverse acc), r)
parseArray s acc = do
  (v, r1) <- parseVal s
  case skip r1 of
    ',' : r2 -> parseArray (skip r2) (v : acc)
    ']' : r2 -> Right (JArray (reverse (v : acc)), r2)
    _        -> Left "expected , or ] in array"

-- | Locate and read the canonical-cbor corpus fixture cases.json.
loadCorpus :: IO [TestCase]
loadCorpus = do
  let candidates =
        [ "../../../specs/fixtures/canonical-cbor-v1/cases.json"
        , "code/specs/fixtures/canonical-cbor-v1/cases.json"
        , "../../../../code/specs/fixtures/canonical-cbor-v1/cases.json"
        ]
  path <- findExisting candidates
  content <- readFile path
  case parseJson content of
    Left err -> ioError (userError ("corpus JSON parse error: " ++ err))
    Right (JObject top) ->
      case lookup "cases" top of
        Just (JArray cs) -> mapM parseCaseItem cs
        _ -> ioError (userError "corpus JSON missing 'cases' array")
    Right _ -> ioError (userError "corpus JSON root is not an object")
  where
    findExisting [] = ioError (userError "could not find cases.json in candidate paths")
    findExisting (p:ps) = do
      exists <- doesFileExist p
      if exists then return p else findExisting ps

    parseCaseItem (JObject fields) =
      case ( lookup "id" fields
           , lookup "operation" fields
           , lookup "input" fields
           , lookup "expected" fields
           ) of
        (Just (JString i), Just (JString op), Just (JString inp), Just (JString expc)) ->
          return (TestCase i op inp expc)
        _ -> ioError (userError "case item missing required fields")
    parseCaseItem _ = ioError (userError "case item is not an object")

hexToBytes :: String -> BS.ByteString
hexToBytes = BS.pack . go
  where
    go (a : b : r) = fromIntegral (digitToInt a * 16 + digitToInt b) : go r
    go [] = []
    go _ = error "odd-length hex string"

bytesToHex :: BS.ByteString -> String
bytesToHex = concatMap byteHex . BS.unpack
  where
    byteHex w = [intToDigit (fromIntegral (w `div` 16)), intToDigit (fromIntegral (w `mod` 16))]
