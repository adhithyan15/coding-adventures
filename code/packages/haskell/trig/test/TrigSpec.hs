module TrigSpec (spec) where

import Data.Either (isLeft)
import Test.Hspec
import Trig
import Prelude hiding (atan, atan2, cos, sin, sqrt, tan)

tolerance :: Double
tolerance = 1e-10

shouldBeCloseTo :: Double -> Double -> Expectation
actual `shouldBeCloseTo` expected =
    abs (actual - expected) `shouldSatisfy` (<= tolerance)

rightValue :: Show error => Either error value -> IO value
rightValue result = case result of
    Left err -> expectationFailure (show err) >> error "unreachable"
    Right value -> pure value

spec :: Spec
spec = do
    describe "constants" $
        it "provides the shared angle values" $ do
            twoPi `shouldBeCloseTo` (2.0 * piValue)
            halfPi `shouldBeCloseTo` (piValue / 2.0)

    describe "sin" $ do
        it "matches special angles" $ do
            let cases =
                    [ (0.0, 0.0)
                    , (piValue / 6.0, 0.5)
                    , (piValue / 4.0, 0.7071067811865476)
                    , (piValue / 3.0, 0.8660254037844386)
                    , (halfPi, 1.0)
                    , (piValue, 0.0)
                    , (3.0 * halfPi, -1.0)
                    , (twoPi, 0.0)
                    ]
            mapM_ (\(angle, expected) -> sin angle `shouldBeCloseTo` expected) cases
        it "is odd" $
            mapM_ (\angle -> sin (-angle) `shouldBeCloseTo` (negate (sin angle)))
                [0.5, 1.0, 1.5, 2.0, 2.7]

    describe "cos" $ do
        it "matches special angles" $ do
            let cases =
                    [ (0.0, 1.0)
                    , (piValue / 6.0, 0.8660254037844386)
                    , (piValue / 4.0, 0.7071067811865476)
                    , (piValue / 3.0, 0.5)
                    , (halfPi, 0.0)
                    , (piValue, -1.0)
                    , (3.0 * halfPi, 0.0)
                    , (twoPi, 1.0)
                    ]
            mapM_ (\(angle, expected) -> cos angle `shouldBeCloseTo` expected) cases
        it "is even" $
            mapM_ (\angle -> cos (-angle) `shouldBeCloseTo` cos angle)
                [0.5, 1.0, 1.5, 2.0, 2.7]

    describe "shared identities" $ do
        it "satisfies the Pythagorean identity" $
            mapM_ checkIdentity
                [0.0, piValue / 6.0, piValue / 4.0, halfPi, piValue, -2.5, 5.5]
        it "range-reduces large positive and negative inputs" $ do
            sin (1000.0 * piValue) `shouldBeCloseTo` 0.0
            cos (1000.0 * piValue) `shouldBeCloseTo` 1.0
            sin (-100.0) `shouldBeCloseTo` (negate (sin 100.0))

    describe "angle conversion" $ do
        it "converts known values" $ do
            radians 180.0 `shouldBeCloseTo` piValue
            radians 90.0 `shouldBeCloseTo` halfPi
            degrees piValue `shouldBeCloseTo` 180.0
            degrees halfPi `shouldBeCloseTo` 90.0
        it "round trips degrees and radians" $ do
            mapM_ (\value -> degrees (radians value) `shouldBeCloseTo` value)
                [0.0, 30.0, 45.0, 90.0, 180.0, 360.0]
            mapM_ (\value -> radians (degrees value) `shouldBeCloseTo` value)
                [0.0, piValue / 6.0, piValue / 4.0, halfPi, piValue, twoPi]

    describe "sqrt" $ do
        it "matches exact and irrational reference values" $ do
            let cases =
                    [ (0.0, 0.0)
                    , (1.0, 1.0)
                    , (4.0, 2.0)
                    , (9.0, 3.0)
                    , (2.0, 1.41421356237)
                    , (0.25, 0.5)
                    , (1e10, 1e5)
                    ]
            mapM_ checkSquareRoot cases
        it "rejects negative input" $
            sqrt (-1.0) `shouldSatisfy` isLeft

    describe "tan" $ do
        it "matches reference values" $ do
            tan 0.0 `shouldBeCloseTo` 0.0
            tan (piValue / 4.0) `shouldBeCloseTo` 1.0
            tan (-piValue / 4.0) `shouldBeCloseTo` (-1.0)
            rootThree <- rightValue $ sqrt 3.0
            tan (piValue / 6.0) `shouldBeCloseTo` (1.0 / rootThree)
        it "returns signed finite sentinels at poles" $ do
            tan halfPi `shouldBe` 1e308
            tan (negate halfPi) `shouldBe` (-1e308)

    describe "atan" $ do
        it "matches reference values and all range branches" $ do
            rootThree <- rightValue $ sqrt 3.0
            atan 0.0 `shouldBeCloseTo` 0.0
            atan 1.0 `shouldBeCloseTo` (piValue / 4.0)
            atan (-1.0) `shouldBeCloseTo` (-piValue / 4.0)
            atan rootThree `shouldBeCloseTo` (piValue / 3.0)
            atan (1.0 / rootThree) `shouldBeCloseTo` (piValue / 6.0)
            abs (atan 1e10 - halfPi) `shouldSatisfy` (<= 1e-5)
            abs (atan (-1e10) + halfPi) `shouldSatisfy` (<= 1e-5)
        it "inverts tangent inside the principal range" $
            atan (tan (piValue / 4.0)) `shouldBeCloseTo` (piValue / 4.0)

    describe "atan2" $ do
        it "handles both axes and the origin" $ do
            atan2 0.0 1.0 `shouldBeCloseTo` 0.0
            atan2 1.0 0.0 `shouldBeCloseTo` halfPi
            atan2 0.0 (-1.0) `shouldBeCloseTo` piValue
            atan2 (-1.0) 0.0 `shouldBeCloseTo` (negate halfPi)
            atan2 0.0 0.0 `shouldBeCloseTo` 0.0
        it "selects all four quadrants" $ do
            atan2 1.0 1.0 `shouldBeCloseTo` (piValue / 4.0)
            atan2 1.0 (-1.0) `shouldBeCloseTo` (3.0 * piValue / 4.0)
            atan2 (-1.0) (-1.0) `shouldBeCloseTo` (-3.0 * piValue / 4.0)
            atan2 (-1.0) 1.0 `shouldBeCloseTo` (-piValue / 4.0)
  where
    checkIdentity angle =
        (sin angle * sin angle + cos angle * cos angle) `shouldBeCloseTo` 1.0
    checkSquareRoot (value, expected) = do
        actual <- rightValue $ sqrt value
        actual `shouldBeCloseTo` expected
