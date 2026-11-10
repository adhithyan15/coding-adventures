-- | Test-only helper: a minimal JSON reader for the CT01 corpus
-- (@code/specs/fixtures/ct-compare-v1/cases.json@).  The library under test
-- has no dependencies beyond @base@ and @bytestring@, and no JSON library
-- ships with GHC, so the tests carry a small decoder for exactly what the
-- corpus uses: objects, arrays, strings, booleans and integers.
module Corpus
  ( Json(..)
  , Case(..)
  , loadCorpus
  , hexToBytes
  , bytesToHex
  , hexToWord64
  ) where

import Data.Char (digitToInt, intToDigit, isDigit, isSpace)
import qualified Data.ByteString as B
import Data.Word (Word64, Word8)
import Numeric (showHex)

data Json
  = JObject [(String, Json)]
  | JArray [Json]
  | JString String
  | JBool Bool
  | JInt Integer
  deriving (Eq, Show)

-- | One corpus record, reduced to what the adapters need.
data Case = Case
  { caseId :: String
  , caseOp :: String
  , caseFields :: [(String, Json)]
  } deriving (Show)

parseJson :: String -> Either String Json
parseJson input = do
  (v, rest) <- value (skip input)
  if null (skip rest) then Right v else Left "trailing content"

skip :: String -> String
skip = dropWhile isSpace

value :: String -> Either String (Json, String)
value ('{' : r) = object (skip r) []
value ('[' : r) = array (skip r) []
value ('"' : r) = do
  (s, rest) <- str r ""
  Right (JString s, rest)
value ('t' : 'r' : 'u' : 'e' : r) = Right (JBool True, r)
value ('f' : 'a' : 'l' : 's' : 'e' : r) = Right (JBool False, r)
value s@(c : _)
  | isDigit c =
      let (ds, rest) = span isDigit s
       in Right (JInt (read ds), rest)
value s = Left ("unexpected JSON: " ++ take 20 s)

str :: String -> String -> Either String (String, String)
str ('"' : r) acc = Right (reverse acc, r)
str ('\\' : c : r) acc = str r (c : acc)
str (c : r) acc = str r (c : acc)
str [] _ = Left "unterminated string"

object :: String -> [(String, Json)] -> Either String (Json, String)
object ('}' : r) acc = Right (JObject (reverse acc), r)
object ('"' : r) acc = do
  (k, r1) <- str r ""
  case skip r1 of
    ':' : r2 -> do
      (v, r3) <- value (skip r2)
      case skip r3 of
        ',' : r4 -> object (skip r4) ((k, v) : acc)
        '}' : r4 -> Right (JObject (reverse ((k, v) : acc)), r4)
        _ -> Left "expected , or }"
    _ -> Left "expected :"
object _ _ = Left "bad object"

array :: String -> [Json] -> Either String (Json, String)
array (']' : r) acc = Right (JArray (reverse acc), r)
array s acc = do
  (v, r1) <- value s
  case skip r1 of
    ',' : r2 -> array (skip r2) (v : acc)
    ']' : r2 -> Right (JArray (reverse (v : acc)), r2)
    _ -> Left "expected , or ]"

-- | Load the shared corpus.  Cabal runs tests from the package directory:
-- @code/packages/haskell/ct-compare@ -> @code@ is three levels up.
loadCorpus :: IO (String, [Case])
loadCorpus = do
  text <- readFile "../../../specs/fixtures/ct-compare-v1/cases.json"
  case parseJson text of
    Left e -> fail ("corpus: " ++ e)
    Right (JObject top) ->
      case (lookup "profile" top, lookup "cases" top) of
        (Just (JString p), Just (JArray cs)) -> do
          cases <- mapM toCase cs
          length text `seq` return (p, cases)
        _ -> fail "corpus: missing profile or cases"
    Right _ -> fail "corpus: not an object"
  where
    toCase (JObject fs) = case (lookup "id" fs, lookup "operation" fs) of
      (Just (JString i), Just (JString o)) -> return (Case i o fs)
      _ -> fail "corpus: case without id/operation"
    toCase _ = fail "corpus: case is not an object"

-- | Decode lowercase hex (even length) into bytes.
hexToBytes :: String -> B.ByteString
hexToBytes = B.pack . go
  where
    go :: String -> [Word8]
    go (a : b : r) = fromIntegral (digitToInt a * 16 + digitToInt b) : go r
    go [] = []
    go _ = error "odd-length hex"

bytesToHex :: B.ByteString -> String
bytesToHex = concatMap byteHex . B.unpack
  where
    byteHex w = [intToDigit (fromIntegral (w `div` 16)), intToDigit (fromIntegral (w `mod` 16))]

-- | Decode exactly 16 hex digits into a 'Word64'.
hexToWord64 :: String -> Word64
hexToWord64 s
  | length s == 16 = foldl (\acc c -> acc * 16 + fromIntegral (digitToInt c)) 0 s
  | otherwise = error ("u64 must be 16 hex digits, got " ++ showHex (length s) "")
