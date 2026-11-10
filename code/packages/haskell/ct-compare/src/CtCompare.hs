{-# LANGUAGE BangPatterns #-}

-- |
-- Module      : CtCompare
-- Description : Constant-time byte comparison and selection (CT01).
--
-- = Why this exists
--
-- A secret comparison that stops at the first differing byte tells an attacker
-- how long the matching prefix was.  Controlling one operand (a MAC tag, a
-- session token), they can recover the other a byte at a time by timing
-- guesses.  The defence is to read EVERY byte whatever the data is, fold the
-- differences into one accumulator, and look at the accumulator once at the
-- end:
--
-- > a   = 41 42 43 44          a   = 41 42 43 44
-- > b   = 41 42 43 44          b   = 41 FF 43 44
-- > xor = 00 00 00 00          xor = 00 BD 00 00   (all four bytes were read)
-- > acc = 0x00 -> equal        acc = 0xBD          -> different
--
-- This is the Haskell lane of CT01 (@code/specs/CT01-constant-time-compare.md@).
--
-- = What this module promises, and what it does not
--
-- The source has no content-dependent early return, branch, index or loop
-- bound; the work depends only on the public lengths.  The accumulator is
-- strict (a bang pattern), so no chain of thunks builds up and no byte is
-- left unread.
--
-- It does __not__ promise wall-clock constant time.  GHC is an optimising
-- compiler: it may rewrite, unroll or vectorise the loop, and nothing in
-- Haskell lets the source pin the machine code.  A garbage-collected runtime
-- can pause at any allocation and copies live data, so secrets can be moved
-- around the heap and left behind in freed memory.  Lazy ByteStrings, @String@
-- and unevaluated thunks are deliberately not accepted: only strict
-- 'ByteString's are.  Inspecting the optimised Core or assembly (for example
-- with @-ddump-simpl@) is the way to check that GHC did not reintroduce an
-- early exit; this package does not claim that it has been done for every
-- compiler version.
--
-- = How the selection bit is received
--
-- Haskell's @Bool@ has no branch-free conversion to a number that the source
-- can name (@if b then 1 else 0@ is a branch on the secret), so, like the Go,
-- Ruby and Elixir lanes, the selection bit is an integer ('Word8') that must
-- be exactly 0 (pick the right input) or 1 (pick the left), and every equality
-- has a bit-returning variant:
--
-- > ctSelectBytes a b (ctEqBit x y)
--
-- never turns a 'Bool' into a bit.
module CtCompare
  ( CtError(..)
  , ctEq
  , ctEqFixed
  , ctSelectBytes
  , ctEqU64
  , ctEqBit
  , ctEqFixedBit
  , ctEqU64Bit
  ) where

import Data.Bits (complement, shiftR, xor, (.&.), (.|.))
import Data.ByteString (ByteString)
import qualified Data.ByteString as B
import qualified Data.ByteString.Unsafe as BU
import Data.Word (Word64, Word8)

-- | The two ways 'ctSelectBytes' can refuse.  Both are decided from public
-- information (the lengths, and the /type-level shape/ of the choice) before
-- either input's contents are read.
data CtError
  = SelectLengthMismatch
    -- ^ CT01's @CT_SELECT_LENGTH_MISMATCH@: the inputs differ in length.
  | InvalidChoice
    -- ^ The selection bit was neither 0 nor 1.
  deriving (Eq, Show)

-- ---------------------------------------------------------------------------
-- Equality
-- ---------------------------------------------------------------------------

-- | OR together the XOR of every byte pair.  Precondition: equal lengths.
--
-- 'BU.unsafeIndex' skips the bounds check (which would be a branch); the index
-- is always below the shared length, so it is safe.  The loop bound @n@ is the
-- public length.
difference :: ByteString -> ByteString -> Word8
difference a b = go 0 0
  where
    n = B.length a
    go :: Int -> Word8 -> Word8
    go !i !acc
      | i == n = acc
      | otherwise =
          go (i + 1) (acc .|. (BU.unsafeIndex a i `xor` BU.unsafeIndex b i))

-- | @1@ when the byte strings are equal, else @0@.  A length mismatch is @0@
-- and reads neither input's contents; lengths are public.
--
-- For an accumulator @acc@ in @0..255@, widened to 64 bits, @acc - 1@ wraps to
-- all ones exactly when @acc == 0@ and is otherwise @0..254@ with the top bit
-- clear.  A logical shift right by 63 therefore gives 1 for "equal" and 0 for
-- "different", with no comparison and no conditional.
ctEqBit :: ByteString -> ByteString -> Word8
ctEqBit a b
  | B.length a /= B.length b = 0
  | otherwise = fromIntegral ((widen (difference a b) - 1) `shiftR` 63)
  where
    widen :: Word8 -> Word64
    widen = fromIntegral

-- | Fixed-width companion of 'ctEqBit'.  Haskell has no fixed-size byte array
-- type in this API, so it is the same operation; it exists so call sites read
-- the same in every CT01 lane.
ctEqFixedBit :: ByteString -> ByteString -> Word8
ctEqFixedBit = ctEqBit

-- | Boolean equality without early exit.  The 'Bool' is the /public/ result.
ctEq :: ByteString -> ByteString -> Bool
ctEq a b = ctEqBit a b == 1

-- | Fixed-width companion of 'ctEq'.
ctEqFixed :: ByteString -> ByteString -> Bool
ctEqFixed = ctEq

-- ---------------------------------------------------------------------------
-- Selection
-- ---------------------------------------------------------------------------

-- | Select the left input when the bit is 1 and the right one when it is 0,
-- returning a freshly allocated 'ByteString' (never an alias of an input).
--
-- With @mask = 0xFF@ (bit 1) or @0x00@ (bit 0):
--
-- > out = right `xor` ((left `xor` right) .&. mask)
--
-- is the left or the right byte by the same instructions either way.  The
-- bit's validity test @bit .&. complement 1 /= 0@ is a single computation with
-- the same answer for both valid values, so a valid call does identical work
-- whichever bit is secret.  Equal lengths are required; the length check comes
-- first and neither check reads any contents.
ctSelectBytes :: ByteString -> ByteString -> Word8 -> Either CtError ByteString
ctSelectBytes left right bit
  | B.length left /= B.length right = Left SelectLengthMismatch
  | bit .&. complement 1 /= 0 = Left InvalidChoice
  | otherwise = Right (B.packZipWith pick left right)
  where
    -- 0 - 0 = 0x00; 0 - 1 wraps to 0xFF.
    mask = 0 - bit
    pick l r = r `xor` ((l `xor` r) .&. mask)

-- ---------------------------------------------------------------------------
-- Unsigned 64-bit equality
-- ---------------------------------------------------------------------------

-- | @1@ when the two values are equal, else @0@.
--
-- @d = x xor y@ is zero exactly when the values are equal.  @d .|. negate d@
-- has its top bit set exactly when @d /= 0@ (for a non-zero @d@ either @d@ or
-- its two's complement has the top bit set), so shifting that bit down and
-- flipping it gives the equality bit.  All 64 bit positions are folded by one
-- fixed operation sequence.
ctEqU64Bit :: Word64 -> Word64 -> Word8
ctEqU64Bit x y = fromIntegral (((d .|. negate d) `shiftR` 63) `xor` 1)
  where
    d = x `xor` y

-- | Boolean form of 'ctEqU64Bit'.
ctEqU64 :: Word64 -> Word64 -> Bool
ctEqU64 x y = ctEqU64Bit x y == 1
