-- | Strict decoding of corpus values (VER05n 8).
--
-- Encoding (JSON):
--
-- > {"kind":"int","value":"-12"}     an Integer (decimal text)
-- > {"kind":"bytes","hex":"00ff"}    a byte string
-- > true / false                     a bool
-- > [ ... ]                          a list of the above
-- > "text"                           a text (types, property names, ids)
--
-- Integers are canonical decimal text of at most 1,000 digits, checked BEFORE
-- the digits are folded into an 'Integer', so a hostile fixture cannot make the
-- decoder allocate a huge number.
module CodingAdventures.TestBootstrap.CorpusJson.Values
  ( decode
  , canonicalInt
  , maxSize
  , maxIntDigits
  , maxFragmentLen
  , maxProblemLen
  , showVal
  ) where

import qualified Data.ByteString as B
import qualified Data.ByteString.Char8 as BC
import Data.List (sort)
import Data.Text (Text)
import qualified Data.Text.Encoding as TE

import CodingAdventures.TestBootstrap.CorpusJson.Canon (render)
import CodingAdventures.TestBootstrap.CorpusJson.Escape (clipText)
import CodingAdventures.TestBootstrap.CorpusJson.StrictJson
import CodingAdventures.TestBootstrap.CorpusJson.Val
import CodingAdventures.TestBootstrap.Util (isLowerHexEven, unhex)

maxSize, maxIntDigits, maxFragmentLen, maxProblemLen :: Int
maxSize = 10000
maxIntDigits = 1000
maxFragmentLen = 200
maxProblemLen = 400

maxValueDepth :: Int
maxValueDepth = 64

-- | Decimal digits only, optional leading @-@, no leading zeros, no @-0@.
canonicalInt :: B.ByteString -> Bool
canonicalInt t = case BC.uncons t of
  Nothing -> False
  Just ('-', rest) -> nonZeroLead rest
  Just _ -> t == "0" || nonZeroLead t
  where
    nonZeroLead d = not (B.null d) && BC.all (\c -> c >= '0' && c <= '9') d && BC.head d /= '0'

-- | Decodes one corpus value; 'Left' is a malformed case (not an engine mismatch).
decode :: Json -> Either String Val
decode = go 0
  where
    go :: Int -> Json -> Either String Val
    go depth node
      | depth > maxValueDepth = Left ("value nesting deeper than " ++ show maxValueDepth)
      | otherwise = case node of
          JBool b -> Right (VBool b)
          JString s -> Right (VText (TE.decodeUtf8 s))
          JArray xs
            | length (take (maxSize + 1) xs) > maxSize -> Left ("list exceeds " ++ show maxSize ++ " elements")
            | otherwise -> VList <$> mapM (go (depth + 1)) xs
          JObject fields -> object fields
          other -> Left ("unsupported JSON value " ++ take 40 (show other))
    object fields = case lookup "kind" fields of
      Just (JString "int") | sort (map fst fields) == ["kind", "value"] -> case lookup "value" fields of
        Just (JString t)
          | not (canonicalInt t) -> Left ("non-canonical int " ++ show (B.take 40 t))
          | B.length t > maxIntDigits + (if BC.head t == '-' then 1 else 0) -> Left ("int exceeds " ++ show maxIntDigits ++ " digits")
          | otherwise -> Right (VInt (readInt t))
        _ -> Left "non-canonical int (value must be a string)"
      Just (JString "bytes") | sort (map fst fields) == ["hex", "kind"] -> case lookup "hex" fields of
        Just (JString t)
          | not (isLowerHexEven t) -> Left ("bytes hex must be even-length lowercase: " ++ show (B.take 40 t))
          | B.length t > 2 * maxSize -> Left ("bytes exceed " ++ show maxSize ++ " bytes")
          | otherwise -> Right (VBytes (unhex t))
        _ -> Left "bytes hex must be a string"
      _ -> Left "unrecognised value node"

-- | Digits to an Integer (callers have checked the digit count and shape).
readInt :: B.ByteString -> Integer
readInt t = case BC.uncons t of
  Just ('-', rest) -> negate (digits rest)
  _ -> digits t
  where
    digits = BC.foldl' (\acc c -> acc * 10 + toInteger (fromEnum c - 48)) 0

-- | A short, bounded rendering for problem lines.
showVal :: Val -> IO Text
showVal v = clipText maxFragmentLen <$> render v
