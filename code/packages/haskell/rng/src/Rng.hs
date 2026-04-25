-- ============================================================================
-- Rng.hs — Three Classic Pseudorandom Number Generators
-- ============================================================================
--
-- A pseudorandom number generator (PRNG) is a deterministic algorithm that
-- takes a small seed value and produces a long sequence of numbers that
-- /looks/ random even though it isn't.  Identical seeds always produce
-- identical sequences — a property that makes unit tests reproducible and
-- simulations replayable.
--
-- This module implements three generators representing 70 years of progress:
--
--   ┌──────────────┬──────┬────────────────────────────────────────────────┐
--   │ Type         │ Year │ Core idea                                      │
--   ├──────────────┼──────┼────────────────────────────────────────────────┤
--   │ LCG          │ 1948 │ multiply-add recurrence; upper 32 bits output  │
--   │ Xorshift64   │ 2003 │ three XOR-shifts; lower 32 bits output         │
--   │ PCG32        │ 2014 │ LCG recurrence + XSH RR output permutation     │
--   └──────────────┴──────┴────────────────────────────────────────────────┘
--
-- All three expose the same five functions through a typeclass:
--
--   nextU32       :: g -> (Word32, g)   — uniform UInt32
--   nextU64       :: g -> (Word64, g)   — uniform UInt64
--   nextFloat     :: g -> (Double, g)   — uniform Double in [0.0, 1.0)
--   nextIntInRange :: Int64 -> Int64 -> g -> (Int64, g)
--
-- Haskell note: @Data.Word.Word64@ and @Data.Word.Word32@ wrap on overflow
-- just like unsigned integers in C or Go — no explicit @mod 2^64@ needed.
--
-- Reference values for seed=1:
--   LCG:        [1817669548, 2187888307, 2784682393]
--   Xorshift64: [1082269761,  201397313, 1854285353]
--   PCG32:      [1412771199, 1791099446,  124312908]
--
-- Layer: CS03 (computer-science layer 3 — leaf package, zero dependencies)
-- Spec:  code/specs/CS03-rng.md
-- ============================================================================

module Rng
    ( -- * Typeclass
      RandomGen(..)
      -- * LCG
    , LCG(..)
    , newLCG
      -- * Xorshift64
    , Xorshift64(..)
    , newXorshift64
      -- * PCG32
    , PCG32(..)
    , newPCG32
    ) where

import Data.Bits   (xor, shiftL, shiftR, rotateR, (.|.))
import Data.Int    (Int64)
import Data.Word   (Word32, Word64)

-- ── Constants ─────────────────────────────────────────────────────────────────

-- | Knuth multiplier for the LCG and PCG32 recurrences.
--
-- Together with 'lcgIncrement', satisfies the Hull-Dobell theorem:
-- @GCD(lcgIncrement, 2^64) = 1@, giving full period 2^64.
lcgMultiplier :: Word64
lcgMultiplier = 6364136223846793005

-- | Additive constant for LCG and PCG32.
--
-- Must be odd for full period.  1442695040888963407 ends in 7 (odd) and was
-- chosen by Knuth for good spectral properties.
lcgIncrement :: Word64
lcgIncrement = 1442695040888963407

-- | Divisor to normalise a 'Word32' output to [0.0, 1.0).
--
-- @2^32 = 4294967296@.  Dividing the raw Word32 by this value maps the
-- maximum possible output (2^32 − 1) to approximately 0.99999999977.
floatDiv :: Double
floatDiv = 4294967296.0  -- 2^32

-- ── Typeclass ─────────────────────────────────────────────────────────────────

-- | A class for pseudorandom number generators.
--
-- All three generators implement this interface.  Haskell generators are
-- pure values — each operation returns a new generator state alongside the
-- output, rather than mutating state in place (as Go and Swift do).
--
-- Example:
--
-- @
--   let g0           = newLCG 1
--       (v1, g1) = nextU32 g0
--       (v2, g2) = nextU32 g1
--       (v3, _)  = nextU32 g2
--   -- v1 = 1817669548, v2 = 2187888307, v3 = 2784682393
-- @
class RandomGen g where
    -- | Advance the generator and return a 32-bit output and the new state.
    nextU32 :: g -> (Word32, g)

    -- | Advance twice and return a 64-bit output: @(hi << 32) | lo@.
    --
    -- Default implementation composes two 'nextU32' calls.
    nextU64 :: g -> (Word64, g)
    nextU64 g0 =
        let (hi, g1) = nextU32 g0
            (lo, g2) = nextU32 g1
        in  ((fromIntegral hi `shiftL` 32) .|. fromIntegral lo, g2)

    -- | Return a 'Double' uniformly distributed in [0.0, 1.0).
    --
    -- Divides the 32-bit output by 2^32.
    nextFloat :: g -> (Double, g)
    nextFloat g0 =
        let (w, g1) = nextU32 g0
        in  (fromIntegral w / floatDiv, g1)

    -- | Return a uniform 'Int64' in [min, max] inclusive.
    --
    -- Uses rejection sampling to eliminate modulo bias.  See the LCG
    -- implementation notes below for a full explanation.
    nextIntInRange :: Int64 -> Int64 -> g -> (Int64, g)
    nextIntInRange lo hi g0
      | lo > hi   = error "nextIntInRange: lo must be <= hi"
      | otherwise =
        let rangeSize = fromIntegral (hi - lo + 1) :: Word64
            threshold = (0 - rangeSize) `mod` rangeSize
            go gen =
                let (r, gen') = nextU32 gen
                    r64       = fromIntegral r :: Word64
                in  if r64 >= threshold
                    then (lo + fromIntegral (r64 `mod` rangeSize), gen')
                    else go gen'
        in  go g0

-- ── LCG ───────────────────────────────────────────────────────────────────────

-- | A Linear Congruential Generator (Knuth 1948).
--
-- == How it works
--
-- Each step advances a 64-bit accumulator via:
--
-- @
--   state = (state × a + c) mod 2^64
-- @
--
-- where a = 6364136223846793005 and c = 1442695040888963407.
-- 'Word64' arithmetic wraps automatically, so no explicit mod is needed.
--
-- The /upper/ 32 bits of state are returned as output.  Low-order bits have
-- shorter sub-periods: bit 0 alternates 0\/1, bit 1 has period 4, etc.
-- Only the upper half carries the full-period guarantee.
--
-- == Strengths and weaknesses
--
--   * (+) Extremely fast: one multiply, one add, one shift
--   * (+) Full period 2^64
--   * (−) Low-order bits correlated
--   * (−) Consecutive pairs fall on hyperplanes (Marsaglia's lattice test)
newtype LCG = LCG { lcgState :: Word64 }
    deriving (Show, Eq)

-- | Create an LCG seeded with the given value.  Any seed is valid.
newLCG :: Word64 -> LCG
newLCG seed = LCG seed

instance RandomGen LCG where
    -- | Advance: state = state * mult + inc; output = upper 32 bits.
    nextU32 (LCG s) =
        let s' = s * lcgMultiplier + lcgIncrement
        in  (fromIntegral (s' `shiftR` 32), LCG s')

-- ── Xorshift64 ────────────────────────────────────────────────────────────────

-- | A Xorshift64 generator (Marsaglia 2003).
--
-- == How it works
--
-- Three XOR-shift operations scramble a 64-bit state:
--
-- @
--   x = x `xor` (x `shiftL` 13)
--   x = x `xor` (x `shiftR`  7)
--   x = x `xor` (x `shiftL` 17)
-- @
--
-- The shift amounts 13, 7, 17 are the unique triple (from Marsaglia's
-- exhaustive 2003 search) for which the 64-bit linear feedback shift
-- register is maximal — visiting all 2^64 − 1 non-zero states.
--
-- The /lower/ 32 bits are returned.  Unlike LCG, quality is more uniform
-- across all bit positions.
--
-- == Zero-seed protection
--
-- State 0 is a fixed point (0 XOR anything = anything, but all bits are 0
-- so every shift gives 0).  Seed 0 is replaced with 1.
newtype Xorshift64 = Xorshift64 { xsState :: Word64 }
    deriving (Show, Eq)

-- | Create an Xorshift64 generator.  Seed 0 is replaced with 1.
newXorshift64 :: Word64 -> Xorshift64
newXorshift64 0    = Xorshift64 1
newXorshift64 seed = Xorshift64 seed

instance RandomGen Xorshift64 where
    -- | Apply the three XOR-shifts; return the lower 32 bits.
    nextU32 (Xorshift64 x0) =
        let x1 = x0 `xor` (x0 `shiftL` 13)
            x2 = x1 `xor` (x1 `shiftR`  7)
            x3 = x2 `xor` (x2 `shiftL` 17)
        in  (fromIntegral x3, Xorshift64 x3)

-- ── PCG32 ─────────────────────────────────────────────────────────────────────

-- | A Permuted Congruential Generator (O'Neill 2014).
--
-- == How it works
--
-- PCG32 uses the same LCG recurrence as 'LCG' but passes the old state
-- through the /XSH RR/ (XOR-Shift High \/ Random Rotate) permutation before
-- returning:
--
-- @
--   xorshifted = fromIntegral (((old `shiftR` 18) `xor` old) `shiftR` 27) :: Word32
--   rot        = fromIntegral (old `shiftR` 59) :: Word32
--   output     = rotateR xorshifted (fromIntegral rot)
-- @
--
-- Step 1 folds the top bits down into a 32-bit value.
-- Step 2 extracts a 5-bit rotation amount from the very top of state.
-- Step 3 rotates by that amount — the rotation distance is unpredictable,
-- which is what breaks the LCG's regularity.
--
-- PCG32 passes all 160 tests in TestU01 BigCrush.  LCG fails dozens;
-- Xorshift64 fails some.
--
-- == Initialisation warm-up
--
-- Low seeds produce poor first outputs if we merely set state = seed.
-- The "initseq" warm-up mirrors the reference C implementation:
--
--   1. Advance once from state=0 (mixes in the increment before adding seed)
--   2. Add seed to state
--   3. Advance once more (scatters seed bits throughout)
data PCG32 = PCG32
    { pcgState     :: !Word64
    , pcgIncrement :: !Word64
    } deriving (Show, Eq)

-- | Create a PCG32 generator with the initseq warm-up.
newPCG32 :: Word64 -> PCG32
newPCG32 seed =
    let inc = lcgIncrement .|. 1          -- must be odd; lcgIncrement is already odd
        -- Step 1: advance from state=0 to incorporate the increment.
        s1  = 0 * lcgMultiplier + inc
        -- Step 2: mix the seed into state.
        s2  = s1 + seed
        -- Step 3: advance once more to scatter seed bits throughout state.
        s3  = s2 * lcgMultiplier + inc
    in  PCG32 { pcgState = s3, pcgIncrement = inc }

instance RandomGen PCG32 where
    -- | Capture old state, advance LCG, then apply XSH RR to old state.
    nextU32 (PCG32 s inc) =
        let -- Advance LCG.
            s'         = s * lcgMultiplier + inc
            -- XSH RR permutation on old state s.
            xorshifted = fromIntegral (((s `shiftR` 18) `xor` s) `shiftR` 27) :: Word32
            rot        = fromIntegral (s `shiftR` 59) :: Int
            output     = rotateR xorshifted rot
        in  (output, PCG32 s' inc)
