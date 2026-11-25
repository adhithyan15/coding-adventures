-- | Small total helpers shared by the engine: the bit length of an
-- 'Integer', strict UTF-8 validation, hex.
module CodingAdventures.TestBootstrap.Util
  ( integerBitLength
  , validUtf8
  , hexLower
  , isLowerHexEven
  , unhex
  ) where

import Data.Bits (shiftR)
import qualified Data.ByteString as B
import qualified Data.ByteString.Char8 as BC
import Data.Word (Word8)

-- | The number of bits of @n@ (0 for 0 and for negatives' magnitude callers
-- pass 'abs').  Found by shifting, never by showing the number: @show@ on a
-- 10-million-digit 'Integer' costs far more than a handful of shifts.
--
-- Worked example: 5 = 0b101.  Doubling finds the first k with @5 >> k == 0@:
-- k = 1 (5>>1 = 2), 2 (5>>2 = 1), 4 (5>>4 = 0), so hi = 4.  Binary search in
-- (2, 4]: mid 3 -> 5>>3 = 0, so the answer is 3.
integerBitLength :: Integer -> Int
integerBitLength n
  | n <= 0 = 0
  | otherwise = search (hi `div` 2) hi
  where
    hi = up 1
    up :: Int -> Int
    up k
      | n `shiftR` k == 0 = k
      | otherwise = up (k * 2)
    -- invariant: n >> lo /= 0 and n >> h == 0
    search :: Int -> Int -> Int
    search lo h
      | h - lo <= 1 = h
      | otherwise =
          let mid = (lo + h) `div` 2
           in if n `shiftR` mid == 0 then search lo mid else search mid h

-- | Strict UTF-8 (RFC 3629): no overlong forms, no surrogates, nothing above
-- U+10FFFF, no truncated sequence.
--
-- > lead byte      continuation bytes
-- > 00..7F         -
-- > C2..DF         80..BF
-- > E0             A0..BF 80..BF        (E0 80..9F would be overlong)
-- > E1..EC, EE..EF 80..BF 80..BF
-- > ED             80..9F 80..BF        (ED A0..BF would be a surrogate)
-- > F0             90..BF 80..BF 80..BF (F0 80..8F would be overlong)
-- > F1..F3         80..BF 80..BF 80..BF
-- > F4             80..8F 80..BF 80..BF (above 8F is > U+10FFFF)
validUtf8 :: B.ByteString -> Bool
validUtf8 = go . B.unpack
  where
    go :: [Word8] -> Bool
    go [] = True
    go (b : rest)
      | b < 0x80 = go rest
      | b >= 0xC2 && b <= 0xDF = conts 1 rest
      | b == 0xE0 = second 0xA0 0xBF 1 rest
      | b >= 0xE1 && b <= 0xEC = conts 2 rest
      | b == 0xED = second 0x80 0x9F 1 rest
      | b >= 0xEE && b <= 0xEF = conts 2 rest
      | b == 0xF0 = second 0x90 0xBF 2 rest
      | b >= 0xF1 && b <= 0xF3 = conts 3 rest
      | b == 0xF4 = second 0x80 0x8F 2 rest
      | otherwise = False
    cont c = c >= 0x80 && c <= 0xBF
    conts :: Int -> [Word8] -> Bool
    conts 0 rest = go rest
    conts k (c : rest) | cont c = conts (k - 1) rest
    conts _ _ = False
    second :: Word8 -> Word8 -> Int -> [Word8] -> Bool
    second lo hi more (c : rest) | c >= lo && c <= hi = conts more rest
    second _ _ _ _ = False

-- | Lowercase hex of a byte string.
hexLower :: B.ByteString -> String
hexLower = concatMap byte . B.unpack
  where
    byte w = [digit (w `div` 16), digit (w `mod` 16)]
    digit d = "0123456789abcdef" !! fromIntegral d

-- | True for even-length text made only of @0-9a-f@.
isLowerHexEven :: B.ByteString -> Bool
isLowerHexEven t = even (B.length t) && BC.all (\c -> (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')) t

-- | Decodes lowercase even-length hex (callers check 'isLowerHexEven' first).
unhex :: B.ByteString -> B.ByteString
unhex t = B.pack (pairs (B.unpack t))
  where
    pairs (a : b : rest) = (val a * 16 + val b) : pairs rest
    pairs _ = []
    val :: Word8 -> Word8
    val c | c >= 0x30 && c <= 0x39 = c - 0x30
          | otherwise = c - 0x61 + 10
