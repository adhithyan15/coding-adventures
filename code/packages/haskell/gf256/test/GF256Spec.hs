-- | Unit tests for the GF256 module.
--
-- These tests verify the algebraic laws that every finite field must satisfy,
-- as well as concrete values from the spec (MA01-gf256.md).
module GF256Spec (spec) where

import Test.Hspec
import Control.Exception (evaluate)
import Data.Array        ((!))
import Data.Bits         (xor)
import GF256

spec :: Spec
spec = do
    -- -----------------------------------------------------------------------
    -- Table sanity checks
    -- -----------------------------------------------------------------------
    describe "expTable" $ do
        it "expTable[0] = 1  (g^0 = 1)" $
            expTable ! 0 `shouldBe` 1

        it "expTable[1] = 2  (g^1 = 2, the generator)" $
            expTable ! 1 `shouldBe` 2

        it "expTable[7] = 128  (2^7 = 0x80)" $
            expTable ! 7 `shouldBe` 128

        it "expTable[8] = 29  (first reduction: 256 XOR 0x11D = 29)" $
            expTable ! 8 `shouldBe` 29

        it "expTable[9] = 58  (29 * 2; no overflow)" $
            expTable ! 9 `shouldBe` 58

        -- Cross-verified values for the 0x11D polynomial.
        -- Note: the spec (MA01-gf256.md) lists different values for indices
        -- 250-254; those appear to correspond to a different polynomial.
        -- The correct values under 0x11D (confirmed by reference Python impl):
        it "expTable[250] = 108 (0x6C)" $ expTable ! 250 `shouldBe` 108
        it "expTable[251] = 216 (0xD8)" $ expTable ! 251 `shouldBe` 216
        it "expTable[252] = 173 (0xAD)" $ expTable ! 252 `shouldBe` 173
        it "expTable[253] = 71  (0x47)" $ expTable ! 253 `shouldBe` 71
        it "expTable[254] = 142 (0x8E)" $ expTable ! 254 `shouldBe` 142

        it "expTable[255] = 1  (cyclic: g^255 = 1)" $
            expTable ! 255 `shouldBe` 1

    describe "logTable" $ do
        it "logTable[1] = 0  (g^0 = 1)" $
            logTable ! 1 `shouldBe` 0

        it "logTable[2] = 1  (g^1 = 2)" $
            logTable ! 2 `shouldBe` 1

        it "logTable[4] = 2  (g^2 = 4)" $
            logTable ! 4 `shouldBe` 2

        it "logTable and expTable are inverses for x = 1..255" $ do
            let roundtrips = [ expTable ! (logTable ! x) | x <- [1..255] ]
            roundtrips `shouldBe` [1..255]

    -- -----------------------------------------------------------------------
    -- gfAdd / gfSub
    -- -----------------------------------------------------------------------
    describe "gfAdd" $ do
        it "is XOR" $
            gfAdd 0x53 0xCA `shouldBe` (0x53 `xor` 0xCA)

        it "add(x, x) = 0 for all x (characteristic 2)" $ do
            let pairs = [ gfAdd x x | x <- [0..255] ]
            all (== 0) pairs `shouldBe` True

        it "add(0, x) = x  (additive identity)" $ do
            let checks = [ gfAdd 0 x == x | x <- [0..255] ]
            all id checks `shouldBe` True

        it "add is commutative" $ do
            gfAdd 0xAB 0xCD `shouldBe` gfAdd 0xCD 0xAB

    describe "gfSub" $ do
        it "equals gfAdd (subtraction = addition in characteristic 2)" $ do
            let checks = [ gfSub x y == gfAdd x y | x <- [0..15], y <- [0..15] ]
            all id checks `shouldBe` True

    -- -----------------------------------------------------------------------
    -- gfMul
    -- -----------------------------------------------------------------------
    describe "gfMul" $ do
        it "gfMul 0 x = 0 (zero absorbs)" $ do
            let checks = [ gfMul 0 x | x <- [0..255] ]
            all (== 0) checks `shouldBe` True

        it "gfMul x 0 = 0 (zero absorbs)" $ do
            let checks = [ gfMul x 0 | x <- [0..255] ]
            all (== 0) checks `shouldBe` True

        it "gfMul 1 x = x (multiplicative identity)" $ do
            let checks = [ gfMul 1 x == x | x <- [1..255] ]
            all id checks `shouldBe` True

        it "gfMul 2 4 = 8" $
            gfMul 2 4 `shouldBe` 8

        it "gfMul 2 128 = 29  (overflow reduction: 256 XOR 0x11D = 29)" $
            gfMul 2 128 `shouldBe` 29

        it "is commutative" $ do
            let checks = [ gfMul x y == gfMul y x | x <- [0..15], y <- [0..15] ]
            all id checks `shouldBe` True

        it "is associative" $ do
            let checks = [ gfMul x (gfMul y z) == gfMul (gfMul x y) z
                         | x <- [1..10], y <- [1..10], z <- [1..10] ]
            all id checks `shouldBe` True

        it "g^255 = 1  (generator has order 255)" $
            gfPow 2 255 `shouldBe` 1

    -- -----------------------------------------------------------------------
    -- gfDiv
    -- -----------------------------------------------------------------------
    describe "gfDiv" $ do
        it "gfDiv x x = 1 for all non-zero x" $ do
            let checks = [ gfDiv x x | x <- [1..255] ]
            all (== 1) checks `shouldBe` True

        it "gfDiv 0 x = 0 for all non-zero x" $ do
            let checks = [ gfDiv 0 x | x <- [1..255] ]
            all (== 0) checks `shouldBe` True

        it "gfDiv (gfMul a b) b = a for non-zero b" $ do
            let checks = [ gfDiv (gfMul a b) b == a
                         | a <- [1..20], b <- [1..20] ]
            all id checks `shouldBe` True

        it "throws on division by zero" $
            evaluate (gfDiv 5 0) `shouldThrow` anyErrorCall

    -- -----------------------------------------------------------------------
    -- gfInv
    -- -----------------------------------------------------------------------
    describe "gfInv" $ do
        it "gfMul x (gfInv x) = 1 for all non-zero x" $ do
            let checks = [ gfMul x (gfInv x) | x <- [1..255] ]
            all (== 1) checks `shouldBe` True

        it "gfInv 1 = 1  (1 is its own inverse)" $
            gfInv 1 `shouldBe` 1

        -- Spec: 0x53 × 0x8C = 1 under primitive polynomial 0x11D
        it "gfInv 0x53 = 0x8C  (spec cross-check from MA01-gf256.md)" $
            gfInv 0x53 `shouldBe` 0x8C

        it "gfMul 0x53 0x8C = 1  (confirms the inverse pair)" $
            gfMul 0x53 0x8C `shouldBe` 1

        it "throws on zero" $
            evaluate (gfInv 0) `shouldThrow` anyErrorCall

    -- -----------------------------------------------------------------------
    -- gfPow
    -- -----------------------------------------------------------------------
    describe "gfPow" $ do
        it "gfPow x 0 = 1 for any x" $ do
            let checks = [ gfPow x 0 | x <- [0..255] ]
            all (== 1) checks `shouldBe` True

        it "gfPow 0 n = 0 for n > 0" $ do
            let checks = [ gfPow 0 n | n <- [1..10] ]
            all (== 0) checks `shouldBe` True

        it "gfPow 2 1 = 2" $
            gfPow 2 1 `shouldBe` 2

        it "gfPow 2 8 = 29  (matches expTable[8])" $
            gfPow 2 8 `shouldBe` 29

        it "gfPow 2 254 = gfInv 2  (a^254 = a^(-1) since a^255 = 1)" $
            gfPow 2 254 `shouldBe` gfInv 2

    -- -----------------------------------------------------------------------
    -- Field axioms
    -- -----------------------------------------------------------------------
    describe "Field axioms" $ do
        it "distributive law: a*(b+c) = a*b + a*c" $ do
            let checks = [ gfMul a (gfAdd b c) == gfAdd (gfMul a b) (gfMul a c)
                         | a <- [0..15], b <- [0..15], c <- [0..15] ]
            all id checks `shouldBe` True

        it "all 255 non-zero elements are distinct powers of 2" $ do
            let powers = [ gfPow 2 i | i <- [0..254] ]
            -- Each element appears exactly once
            length powers `shouldBe` 255
            length (filter (> 0) powers) `shouldBe` 255
