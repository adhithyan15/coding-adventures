-- ============================================================================
-- RngSpec.hs — Hspec tests for Rng module
-- ============================================================================
--
-- Reference values (seed = 1, from Go implementation):
--   LCG:        [1817669548, 2187888307, 2784682393]
--   Xorshift64: [1082269761,  201397313, 1854285353]
--   PCG32:      [1412771199, 1791099446,  124312908]
--
-- Test sections:
--   1. Known-value tests  — deterministic correctness vs Go reference
--   2. API shape tests    — return types, unit-interval bounds
--   3. Statistical tests  — die coverage, float mean
--   4. Edge-case tests    — seed 0, range=1, reproducibility
-- ============================================================================

module RngSpec (spec) where

import Test.Hspec
import Data.Bits (shiftL, (.|.))
import Data.Int  (Int64)
import Data.Word (Word32, Word64)
import Data.List (nub, sort)

import Rng

-- ── Helpers ───────────────────────────────────────────────────────────────────

-- | Draw n consecutive Word32 values from a generator.
drawU32 :: RandomGen g => Int -> g -> [Word32]
drawU32 0 _ = []
drawU32 n g =
    let (v, g') = nextU32 g
    in  v : drawU32 (n - 1) g'

-- | Draw n consecutive Double values.
drawFloat :: RandomGen g => Int -> g -> [Double]
drawFloat 0 _ = []
drawFloat n g =
    let (v, g') = nextFloat g
    in  v : drawFloat (n - 1) g'

-- | Draw n consecutive Int64 values in [lo, hi].
drawRange :: RandomGen g => Int64 -> Int64 -> Int -> g -> [Int64]
drawRange _  _  0 _ = []
drawRange lo hi n g =
    let (v, g') = nextIntInRange lo hi g
    in  v : drawRange lo hi (n - 1) g'

-- | Reconstruct a Word64 from two Word32 halves: (hi << 32) | lo.
combineHiLo :: Word32 -> Word32 -> Word64
combineHiLo hi lo =
    (fromIntegral hi `shiftL` 32) .|. fromIntegral lo

-- ── Spec ──────────────────────────────────────────────────────────────────────

spec :: Spec
spec = do

    -- ========================================================================
    -- 1. LCG known-value tests
    -- ========================================================================
    describe "LCG" $ do

        it "produces reference values for seed=1" $ do
            let g    = newLCG 1
                vals = drawU32 3 g
            vals `shouldBe` [1817669548, 2187888307, 2784682393]

        it "seed=0 does not crash and returns a deterministic value" $ do
            -- After one step: s' = 0 * mult + inc = lcgIncrement
            -- Output = upper 32 bits of lcgIncrement
            let g      = newLCG 0
                (v, _) = nextU32 g
                inc    = 1442695040888963407 :: Word64
                expected = fromIntegral (inc `shiftR32` 32)
            v `shouldBe` expected

        it "is reproducible: same seed produces same sequence" $ do
            let v1 = drawU32 20 (newLCG 99999)
                v2 = drawU32 20 (newLCG 99999)
            v1 `shouldBe` v2

        it "different seeds produce different first outputs" $ do
            let (v1, _) = nextU32 (newLCG 1)
                (v2, _) = nextU32 (newLCG 2)
            v1 `shouldNotBe` v2

        it "nextU64 equals (hi << 32) | lo from two consecutive nextU32" $ do
            let g0           = newLCG 42
                (hi, g1)     = nextU32 g0
                (lo, _)      = nextU32 g1
                expected     = combineHiLo hi lo
                (u64, _)     = nextU64 g0
            u64 `shouldBe` expected

        it "nextFloat returns values in [0.0, 1.0)" $ do
            let vals = drawFloat 1000 (newLCG 7)
            all (\f -> f >= 0.0 && f < 1.0) vals `shouldBe` True

        it "nextIntInRange stays within [min, max]" $ do
            let vals = drawRange (-5) 5 2000 (newLCG 42)
            all (\v -> v >= -5 && v <= 5) vals `shouldBe` True

        it "nextIntInRange with range=1 always returns min" $ do
            let vals = drawRange 42 42 100 (newLCG 0)
            all (== 42) vals `shouldBe` True

        it "covers all faces of a die in 1200 rolls" $ do
            let vals = sort . nub $ drawRange 1 6 1200 (newLCG 42)
            vals `shouldBe` [1, 2, 3, 4, 5, 6]

    -- ========================================================================
    -- 2. Xorshift64 known-value tests
    -- ========================================================================
    describe "Xorshift64" $ do

        it "produces reference values for seed=1" $ do
            let g    = newXorshift64 1
                vals = drawU32 3 g
            vals `shouldBe` [1082269761, 201397313, 1854285353]

        it "seed=0 is replaced with 1 (same output as seed=1)" $ do
            let v0 = drawU32 10 (newXorshift64 0)
                v1 = drawU32 10 (newXorshift64 1)
            v0 `shouldBe` v1

        it "is reproducible" $ do
            let v1 = drawU32 20 (newXorshift64 123456789)
                v2 = drawU32 20 (newXorshift64 123456789)
            v1 `shouldBe` v2

        it "different seeds diverge" $ do
            let (a, _) = nextU32 (newXorshift64 100)
                (b, _) = nextU32 (newXorshift64 200)
            a `shouldNotBe` b

        it "nextU64 equals (hi << 32) | lo" $ do
            let g0       = newXorshift64 42
                (hi, g1) = nextU32 g0
                (lo, _)  = nextU32 g1
                expected = combineHiLo hi lo
                (u64, _) = nextU64 g0
            u64 `shouldBe` expected

        it "nextFloat returns values in [0.0, 1.0)" $ do
            let vals = drawFloat 1000 (newXorshift64 31415926)
            all (\f -> f >= 0.0 && f < 1.0) vals `shouldBe` True

        it "nextIntInRange stays within bounds" $ do
            let vals = drawRange 1 6 2000 (newXorshift64 42)
            all (\v -> v >= 1 && v <= 6) vals `shouldBe` True

        it "covers all values in [0, 9] over 2000 draws" $ do
            let vals = sort . nub $ drawRange 0 9 2000 (newXorshift64 314159)
            vals `shouldBe` [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]

    -- ========================================================================
    -- 3. PCG32 known-value tests
    -- ========================================================================
    describe "PCG32" $ do

        it "produces reference values for seed=1" $ do
            let g    = newPCG32 1
                vals = drawU32 3 g
            vals `shouldBe` [1412771199, 1791099446, 124312908]

        it "is reproducible" $ do
            let v1 = drawU32 20 (newPCG32 7777)
                v2 = drawU32 20 (newPCG32 7777)
            v1 `shouldBe` v2

        it "seed=0 differs from seed=1" $ do
            let v0 = drawU32 3 (newPCG32 0)
                v1 = drawU32 3 (newPCG32 1)
            v0 `shouldNotBe` v1

        it "nextU64 equals (hi << 32) | lo" $ do
            let g0       = newPCG32 99
                (hi, g1) = nextU32 g0
                (lo, _)  = nextU32 g1
                expected = combineHiLo hi lo
                (u64, _) = nextU64 g0
            u64 `shouldBe` expected

        it "nextFloat returns values in [0.0, 1.0)" $ do
            let vals = drawFloat 1000 (newPCG32 2718281828)
            all (\f -> f >= 0.0 && f < 1.0) vals `shouldBe` True

        it "nextIntInRange stays within [0, 100]" $ do
            let vals = drawRange 0 100 2000 (newPCG32 42)
            all (\v -> v >= 0 && v <= 100) vals `shouldBe` True

        it "nextIntInRange with range=1 always returns min" $ do
            let vals = drawRange (-7) (-7) 100 (newPCG32 0)
            all (== -7) vals `shouldBe` True

        it "covers all faces of a die in 1200 rolls" $ do
            let vals = sort . nub $ drawRange 1 6 1200 (newPCG32 99)
            vals `shouldBe` [1, 2, 3, 4, 5, 6]

    -- ========================================================================
    -- 4. Statistical sanity tests
    -- ========================================================================
    describe "Statistical sanity" $ do

        it "PCG32 float mean is near 0.5 over 10000 draws" $ do
            let vals = drawFloat 10000 (newPCG32 12345)
                mean = sum vals / fromIntegral (length vals)
            (mean > 0.48 && mean < 0.52) `shouldBe` True

        it "LCG float mean is near 0.5 over 10000 draws" $ do
            let vals = drawFloat 10000 (newLCG 12345)
                mean = sum vals / fromIntegral (length vals)
            (mean > 0.47 && mean < 0.53) `shouldBe` True

        it "all three generators produce different first values for seed=1" $ do
            let (a, _) = nextU32 (newLCG 1)
                (b, _) = nextU32 (newXorshift64 1)
                (c, _) = nextU32 (newPCG32 1)
            (a /= b && b /= c && a /= c) `shouldBe` True

        it "PCG32 negative range [(-100), 100] stays in bounds" $ do
            let vals = drawRange (-100) 100 2000 (newPCG32 55)
            all (\v -> v >= -100 && v <= 100) vals `shouldBe` True

-- ── Local helpers (avoid unsafe bit ops) ──────────────────────────────────────

shiftR32 :: Word64 -> Int -> Word64
shiftR32 w n = w `div` (2 ^ n)
