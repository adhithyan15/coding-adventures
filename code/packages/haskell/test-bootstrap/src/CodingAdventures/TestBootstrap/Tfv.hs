-- | Strict validator for TFV v1 text, the line format that carries test corpora
-- between languages (TF01 section 2).  This is a VALIDATOR, not a reader: its one
-- job here is to let the corpus runner check the pbt-v1 @corpus_reject@ cases
-- with a real parser instead of skipping them.
--
-- > tfv 1
-- > # comment
-- > <id> R<n> K<field> <value> K<field> <value> ...
--
-- A value is one of
--
-- > N  T  F                 null, true, false
-- > I<int>                  canonical decimal integer, at most 1000 digits
-- > D<16 hex>              IEEE-754 bit pattern, lowercase
-- > X<hex>  S<hex>          bytes / UTF-8 text as even-length lowercase hex
-- > L<n> v1 ... vn          list
-- > M<n> k1 v1 ... kn vn    map; no two keys deep-equal
--
-- Tokens are separated by exactly one ASCII space.  Rejected: uppercase or
-- odd-length hex, @I-0@, @I007@, a count that disagrees with the tokens,
-- trailing tokens, doubled spaces, a duplicate field or record id, invalid UTF-8
-- inside @S@, a carriage return in a record line, nesting deeper than 64
-- containers, integers of more than 1000 digits, more than 1 MiB of input and
-- more than 200,000 tokens.  A count is NEVER used to reserve memory.
--
-- Worked example: @"tfv 1\\nr R1 Kop S706274 Kdata XFF\\n"@ fails (@XFF@ is
-- uppercase and odd length); @"bad header\\n"@ fails (the first line must be
-- exactly @tfv 1@).
module CodingAdventures.TestBootstrap.Tfv
  ( validate
  , valid
  , maxTfvInput
  , maxTfvTokens
  ) where

import Control.Monad (foldM, unless, when)
import qualified Data.ByteString as B
import qualified Data.ByteString.Char8 as BC
import qualified Data.Set as Set
import Data.Word (Word8)

import CodingAdventures.TestBootstrap.Util (isLowerHexEven, unhex, validUtf8)

maxTfvInput, maxTfvTokens, maxDepth, maxIntDigits :: Int
maxTfvInput = 1048576
maxTfvTokens = 200000
maxDepth = 64
maxIntDigits = 1000

valid :: B.ByteString -> Bool
valid = either (const False) (const True) . validate

type E = Either String

-- | 'Right' for a well-formed document, 'Left' with a reason otherwise.
validate :: B.ByteString -> E ()
validate input = do
  when (B.length input > maxTfvInput) (Left ("input of " ++ show (B.length input) ++ " bytes exceeds " ++ show maxTfvInput))
  unless (validUtf8 input) (Left "document is not valid UTF-8")
  let ls0 = BC.split '\n' input
      ls = if not (null ls0) && B.null (last ls0) then init ls0 else ls0
  case ls of
    (h : _) | h == "tfv 1" -> pure ()
    _ -> Left "the first line must be exactly \"tfv 1\""
  _ <- foldM record (Set.empty, 0 :: Int) (zip [2 :: Int ..] (drop 1 ls))
  pure ()
  where
    record (seen, toks) (n, line)
      | B.null line || BC.head line == '#' = pure (seen, toks)
      | otherwise = case validateRecord line of
          Left e -> Left ("line " ++ show n ++ ": " ++ e)
          Right (rid, used) -> do
            when (Set.member rid seen) (Left ("line " ++ show n ++ ": duplicate record id"))
            when (toks + used > maxTfvTokens) (Left ("more than " ++ show maxTfvTokens ++ " tokens"))
            pure (Set.insert rid seen, toks + used)

validRecordId :: B.ByteString -> Bool
validRecordId t = not (B.null t) && BC.all ok t
  where
    ok c = (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c `elem` ("._/-" :: String)

validFieldName :: B.ByteString -> Bool
validFieldName t = case BC.uncons t of
  Just (c, rest) -> c >= 'a' && c <= 'z' && BC.all (\x -> (x >= 'a' && x <= 'z') || (x >= '0' && x <= '9') || x == '_') rest
  Nothing -> False

-- | A count: canonical digits, at most 9.
canonicalNat :: B.ByteString -> E Int
canonicalNat t
  | B.null t = Left "empty count"
  | not (BC.all (\c -> c >= '0' && c <= '9') t) = Left "non-digit in count"
  | B.length t > 1 && BC.head t == '0' = Left "leading zero in count"
  | B.length t > 9 = Left "count too large"
  | otherwise = Right (read (BC.unpack t))

validInt :: B.ByteString -> E ()
validInt body = do
  let (neg, digits) = case BC.uncons body of
        Just ('-', rest) -> (True, rest)
        _ -> (False, body)
  when (B.null digits) (Left "empty integer")
  unless (BC.all (\c -> c >= '0' && c <= '9') digits) (Left "non-digit in integer")
  when (BC.head digits == '0' && (B.length digits > 1 || neg)) (Left "non-canonical integer")
  when (B.length digits > maxIntDigits) (Left ("integer of " ++ show (B.length digits) ++ " digits exceeds " ++ show maxIntDigits))

-- | One record line: its id and the number of tokens it used.
validateRecord :: B.ByteString -> E (B.ByteString, Int)
validateRecord line = do
  when (BC.elem '\r' line) (Left "carriage return in a record line")
  let tokens = BC.split ' ' line
  when (any B.null tokens) (Left "leading, trailing or doubled space")
  case tokens of
    (rid : rn : rest) -> do
      unless (validRecordId rid) (Left "bad record id")
      unless (BC.head rn == 'R') (Left "expected R<n>")
      count <- canonicalNat (B.drop 1 rn)
      (left, _) <- fields count rest Set.empty
      unless (null left) (Left "trailing tokens after the last field")
      pure (rid, length tokens)
    _ -> Left "a record needs an id and R<n>"
  where
    fields :: Int -> [B.ByteString] -> Set.Set B.ByteString -> E ([B.ByteString], Set.Set B.ByteString)
    fields 0 ts seen = Right (ts, seen)
    fields k ts seen = case ts of
      [] -> Left "record ends early"
      (key : rest) -> do
        unless (BC.head key == 'K' && validFieldName (B.drop 1 key)) (Left "bad field key")
        when (Set.member key seen) (Left "duplicate field")
        (rest', _) <- value 0 rest
        fields (k - 1) rest' (Set.insert key seen)

-- | Reads one value; returns the remaining tokens and a deep-equality key (equal
-- for two values exactly when TF01 3.1 calls them deep-equal).
value :: Int -> [B.ByteString] -> E ([B.ByteString], B.ByteString)
value _ [] = Left "record ends early"
value depth (tok : rest) =
  let tag = BC.head tok
      body = B.drop 1 tok
   in case tag of
        _ | tag `elem` ("NTF" :: String) -> do
              unless (B.null body) (Left "unexpected body")
              pure (rest, tok)
        'I' -> validInt body >> pure (rest, tok)
        'D' -> do
          unless (B.length body == 16 && isLowerHexEven body) (Left "float needs exactly 16 lowercase hex digits")
          let raw = B.unpack (unhex body)
          pure (rest, floatKey tok raw)
        'X' -> do
          unless (isLowerHexEven body) (Left "bytes need even-length lowercase hex")
          pure (rest, tok)
        'S' -> do
          unless (isLowerHexEven body) (Left "text needs even-length lowercase hex")
          unless (validUtf8 (unhex body)) (Left "text is not strict UTF-8")
          pure (rest, tok)
        'L' -> container True body depth rest
        'M' -> container False body depth rest
        _ -> Left "unknown tag"

-- | Every NaN equals every NaN; -0 and +0 stay distinct.
floatKey :: B.ByteString -> [Word8] -> B.ByteString
floatKey tok raw = case raw of
  (b0 : b1 : more) ->
    let ex = ((fromIntegral (b0 `mod` 128) :: Int) * 16) + fromIntegral (b1 `div` 16)
        mantissaZero = b1 `mod` 16 == 0 && all (== 0) more
     in if ex == 0x7ff && not mantissaZero then "Dnan" else tok
  _ -> tok

container :: Bool -> B.ByteString -> Int -> [B.ByteString] -> E ([B.ByteString], B.ByteString)
container isList body depth rest = do
  when (depth + 1 > maxDepth) (Left ("nesting deeper than " ++ show maxDepth))
  count <- canonicalNat body
  if isList
    then do
      (rest', keys) <- items count rest []
      pure (rest', B.concat ["L[", B.intercalate "," (reverse keys), ",]"])
    else do
      (rest', entries, _) <- pairs count rest [] Set.empty
      pure (rest', B.concat ["M{", B.concat (sortBS entries), "}"])
  where
    items :: Int -> [B.ByteString] -> [B.ByteString] -> E ([B.ByteString], [B.ByteString])
    items 0 ts acc = Right (ts, acc)
    items k ts acc = do
      (ts', key) <- value (depth + 1) ts
      items (k - 1) ts' (key : acc)
    pairs :: Int -> [B.ByteString] -> [B.ByteString] -> Set.Set B.ByteString -> E ([B.ByteString], [B.ByteString], ())
    pairs 0 ts acc _ = Right (ts, acc, ())
    pairs k ts acc keys = do
      (ts1, kk) <- value (depth + 1) ts
      (ts2, vv) <- value (depth + 1) ts1
      when (Set.member kk keys) (Left "duplicate map key")
      pairs (k - 1) ts2 (B.concat ["(", kk, "=>", vv, ")"] : acc) (Set.insert kk keys)
    sortBS = Set.toAscList . Set.fromList
