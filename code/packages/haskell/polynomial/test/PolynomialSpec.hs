-- | Unit tests for the Polynomial module.
--
-- These tests verify algebraic laws for GF(256) polynomial arithmetic,
-- matching the spec (MA00-polynomial.md) and TypeScript reference
-- implementation behavior.
module PolynomialSpec (spec) where

import Test.Hspec
import Control.Exception (evaluate)
import Polynomial
import GF256 (gfMul, gfAdd)

spec :: Spec
spec = do
    -- -----------------------------------------------------------------------
    -- Normalization and degree
    -- -----------------------------------------------------------------------
    describe "polyNormalize" $ do
        it "strips trailing zeros" $
            polyNormalize (Poly [1, 0, 0]) `shouldBe` Poly [1]

        it "zero polynomial normalizes to []" $
            polyNormalize (Poly [0]) `shouldBe` Poly []

        it "empty list stays empty" $
            polyNormalize (Poly []) `shouldBe` Poly []

        it "already-normalized polynomial unchanged" $
            polyNormalize (Poly [1, 2, 3]) `shouldBe` Poly [1, 2, 3]

    describe "polyDegree" $ do
        it "degree [] = -1  (zero polynomial)" $
            polyDegree (Poly []) `shouldBe` -1

        it "degree [0] = -1  (normalizes to zero)" $
            polyDegree (Poly [0]) `shouldBe` -1

        it "degree [7] = 0  (constant polynomial)" $
            polyDegree (Poly [7]) `shouldBe` 0

        it "degree [3, 0, 2] = 2" $
            polyDegree (Poly [3, 0, 2]) `shouldBe` 2

        it "degree [1, 2, 3] = 2" $
            polyDegree (Poly [1, 2, 3]) `shouldBe` 2

    -- -----------------------------------------------------------------------
    -- Identity polynomials
    -- -----------------------------------------------------------------------
    describe "polyZero / polyOne" $ do
        it "polyZero = Poly []" $
            polyZero `shouldBe` Poly []

        it "polyOne = Poly [1]" $
            polyOne `shouldBe` Poly [1]

    -- -----------------------------------------------------------------------
    -- polyAdd over GF(256)
    -- -----------------------------------------------------------------------
    describe "polyAdd" $ do
        it "add same polynomial = zero (characteristic 2: a XOR a = 0)" $ do
            let p = Poly [1, 2, 3]
            polyAdd p p `shouldBe` polyZero

        it "add with polyZero = identity" $ do
            let p = Poly [1, 2, 3]
            polyAdd p polyZero `shouldBe` p

        it "add is commutative" $ do
            let p = Poly [1, 2, 3]
                q = Poly [4, 5]
            polyAdd p q `shouldBe` polyAdd q p

        it "add polynomials of different lengths" $ do
            -- [1,2,3] + [4,5] = [1 XOR 4, 2 XOR 5, 3] = [5, 7, 3]
            polyAdd (Poly [1, 2, 3]) (Poly [4, 5])
                `shouldBe` Poly [5, 7, 3]

        it "add with GF(256) coefficient arithmetic" $ do
            -- [0x53, 0x8C] + [0x8C, 0x53]
            -- = [0x53 XOR 0x8C, 0x8C XOR 0x53]
            -- = [gfAdd 0x53 0x8C, gfAdd 0x8C 0x53]
            let a = Poly [0x53, 0x8C]
                b = Poly [0x8C, 0x53]
                expected = Poly [gfAdd 0x53 0x8C, gfAdd 0x8C 0x53]
            polyAdd a b `shouldBe` expected

    -- -----------------------------------------------------------------------
    -- polySub over GF(256)
    -- -----------------------------------------------------------------------
    describe "polySub" $ do
        it "sub = add in GF(256) (characteristic 2)" $ do
            let p = Poly [1, 2, 3]
                q = Poly [4, 5, 6]
            polySub p q `shouldBe` polyAdd p q

        it "sub p p = polyZero" $ do
            let p = Poly [5, 6, 7]
            polySub p p `shouldBe` polyZero

    -- -----------------------------------------------------------------------
    -- polyMul over GF(256)
    -- -----------------------------------------------------------------------
    describe "polyMul" $ do
        it "mul by polyZero = polyZero" $ do
            polyMul (Poly [1, 2, 3]) polyZero `shouldBe` polyZero
            polyMul polyZero (Poly [1, 2, 3]) `shouldBe` polyZero

        it "mul by polyOne = identity" $ do
            let p = Poly [1, 2, 3]
            polyMul p polyOne `shouldBe` p

        it "degree of product = sum of degrees" $ do
            let p = Poly [1, 2]    -- degree 1
                q = Poly [3, 4, 5] -- degree 2
            polyDegree (polyMul p q) `shouldBe` 3

        it "linear factors: (x + a)(x + b) over GF(256)" $ do
            -- (x + 2)(x + 4) = x^2 + (2 XOR 4)x + gfMul 2 4
            --                 = x^2 + 6x + 8
            -- Little-endian: [8, 6, 1]
            let a = Poly [2, 1]  -- 2 + x = (x + 2)
                b = Poly [4, 1]  -- 4 + x = (x + 4)
            polyMul a b `shouldBe` Poly [gfMul 2 4, gfAdd 2 4, 1]

        it "generator n_check=2: (x+2)(x+4) = [8,6,1]" $ do
            let f1 = Poly [2, 1]
                f2 = Poly [4, 1]
            polyMul f1 f2 `shouldBe` Poly [8, 6, 1]

        it "is commutative" $ do
            let p = Poly [1, 2, 3]
                q = Poly [4, 5]
            polyMul p q `shouldBe` polyMul q p

    -- -----------------------------------------------------------------------
    -- polyDivMod over GF(256)
    -- -----------------------------------------------------------------------
    describe "polyDivMod" $ do
        it "a = b * q + r for degree-3 / degree-1" $ do
            -- Use [2, 1] as divisor (x + 2)
            let a = Poly [1, 2, 3, 4]
                b = Poly [2, 1]
                (q, r) = polyDivMod a b
                reconstructed = polyAdd (polyMul b q) r
            polyNormalize reconstructed `shouldBe` polyNormalize a

        it "remainder degree < divisor degree" $ do
            let a = Poly [1, 2, 3, 4]
                b = Poly [2, 1]
                (_, r) = polyDivMod a b
            polyDegree r `shouldSatisfy` (< polyDegree b)

        it "divides evenly when remainder is 0" $ do
            -- (x + 2)(x + 4) = [8, 6, 1]
            -- [8, 6, 1] / (x + 2) = (x + 4) with remainder 0
            let product = Poly [8, 6, 1]
                divisor = Poly [2, 1]
                (q, r)  = polyDivMod product divisor
            r `shouldBe` polyZero
            q `shouldBe` Poly [4, 1]

        it "throws on division by zero polynomial" $
            evaluate (polyDivMod (Poly [1]) (Poly [])) `shouldThrow` anyErrorCall

        it "quotient * divisor + remainder = dividend (round-trip)" $ do
            let a = Poly [1, 2, 3, 4, 5]
                b = Poly [1, 2, 3]
                (q, r) = polyDivMod a b
            polyAdd (polyMul b q) r `shouldBe` polyNormalize a

    -- -----------------------------------------------------------------------
    -- polyEval (Horner's method over GF(256))
    -- -----------------------------------------------------------------------
    describe "polyEval" $ do
        it "zero polynomial evaluates to 0 everywhere" $
            polyEval polyZero 42 `shouldBe` 0

        it "constant polynomial evaluates to the constant" $
            polyEval (Poly [7]) 42 `shouldBe` 7

        it "evaluates [1] at any x = 1" $
            polyEval polyOne 42 `shouldBe` 1

        it "evaluates at 0 returns constant term" $ do
            polyEval (Poly [5, 3, 2]) 0 `shouldBe` 5

        it "generator [8,6,1] at root alpha^1=2 should be 0" $ do
            -- g(x) = x^2 + 6x + 8; root is alpha^1 = 2
            -- g(2) = 4 + 12 + 8 (GF256) = gfMul 1 4 + gfMul 6 2 + 8
            --      = 4 XOR gfMul 6 2 XOR 8
            -- gfMul 6 2 = ?
            -- logTable[6] = ?, logTable[2] = 1
            -- Actually: 4 XOR (6*2 in GF) XOR 8
            -- Let's just assert the root property
            let gen = Poly [8, 6, 1]
            polyEval gen 2 `shouldBe` 0

        it "generator [8,6,1] at root alpha^2=4 should be 0" $ do
            let gen = Poly [8, 6, 1]
            polyEval gen 4 `shouldBe` 0

    -- -----------------------------------------------------------------------
    -- polyGcd over GF(256)
    -- -----------------------------------------------------------------------
    describe "polyGcd" $ do
        it "gcd(p, polyZero) = p" $ do
            let p = Poly [1, 2, 3]
            polyNormalize (polyGcd p polyZero) `shouldBe` polyNormalize p

        it "gcd(polyZero, p) = p" $ do
            let p = Poly [1, 2, 3]
            polyNormalize (polyGcd polyZero p) `shouldBe` polyNormalize p

        it "gcd(p, p) = p" $ do
            let p = Poly [2, 1]
            polyNormalize (polyGcd p p) `shouldBe` polyNormalize p

        it "gcd of product and factor divides the product" $ do
            let f1  = Poly [2, 1]  -- (x + 2)
                f2  = Poly [4, 1]  -- (x + 4)
                prod = polyMul f1 f2
                g   = polyGcd prod f1
                r   = polyMod f1 g
            r `shouldBe` polyZero

    -- -----------------------------------------------------------------------
    -- polyScale helper
    -- -----------------------------------------------------------------------
    describe "polyScale" $ do
        it "scale by 0 = polyZero" $ do
            polyScale 0 (Poly [1, 2, 3]) `shouldBe` polyZero

        it "scale by 1 = identity" $ do
            let p = Poly [1, 2, 3]
            polyScale 1 p `shouldBe` p

        it "scale by a coefficient multiplies each term" $ do
            let s = 2
                p = Poly [4, 8]
            polyScale s p `shouldBe` Poly [gfMul s 4, gfMul s 8]
