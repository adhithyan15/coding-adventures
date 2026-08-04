module WaveSpec (spec) where

import Data.Either (isLeft)
import qualified Control.Exception as Exception
import Test.Hspec
import qualified Trig
import Wave

tolerance :: Double
tolerance = 1e-10

maxFinite :: Double
maxFinite = 1.7976931348623157e308

shouldBeCloseTo :: Double -> Double -> Expectation
actual `shouldBeCloseTo` expected =
    abs (actual - expected) `shouldSatisfy` (<= tolerance)

rightValue :: Show error => Either error value -> IO value
rightValue result = case result of
    Left err -> expectationFailure (show err) >> error "unreachable"
    Right value -> pure value

spec :: Spec
spec = do
    describe "construction" $ do
        it "defaults phase to zero" $ do
            wave <- rightValue $ newWave 1.0 1.0
            phase wave `shouldBeCloseTo` 0.0
        it "stores explicit parameters" $ do
            wave <- rightValue $ newWaveWithPhase 3.5 2.0 1.25
            amplitude wave `shouldBeCloseTo` 3.5
            frequency wave `shouldBeCloseTo` 2.0
            phase wave `shouldBeCloseTo` 1.25
        it "allows zero amplitude" $ do
            wave <- rightValue $ newWave 0.0 1.0
            amplitude wave `shouldBeCloseTo` 0.0
        it "has an inspectable representation" $ do
            wave <- rightValue $ newWaveWithPhase 2.0 3.0 0.5
            same <- rightValue $ newWaveWithPhase 2.0 3.0 0.5
            different <- rightValue $ newWaveWithPhase 2.0 3.0 0.25
            wave `shouldBe` same
            wave `shouldNotBe` different
            show wave `shouldBe`
                "Wave {amplitude = 2.0, frequency = 3.0, phase = 0.5}"

    describe "validation" $ do
        it "rejects negative amplitude" $
            newWave (-1.0) 1.0 `shouldSatisfy` isLeft
        it "rejects zero frequency" $
            newWave 1.0 0.0 `shouldSatisfy` isLeft
        it "rejects negative frequency" $
            newWave 1.0 (-5.0) `shouldSatisfy` isLeft
        it "rejects nonfinite parameters and angular overflow" $ do
            let infinity = 1.0 / 0.0
                nan = 0.0 / 0.0
                invalid =
                    [ newWave nan 1.0
                    , newWave infinity 1.0
                    , newWave 1.0 nan
                    , newWave 1.0 infinity
                    , newWave 1.0 maxFinite
                    , newWaveWithPhase 1.0 1.0 nan
                    , newWaveWithPhase 1.0 1.0 infinity
                    ]
            invalid `shouldSatisfy` all isLeft

    describe "derived quantities" $ do
        it "computes period" $ do
            wave <- rightValue $ newWave 1.0 4.0
            period wave `shouldBeCloseTo` 0.25
        it "computes angular frequency" $ do
            wave <- rightValue $ newWave 1.0 10.0
            angularFrequency wave `shouldBeCloseTo` (2.0 * Trig.piValue * 10.0)

    describe "evaluate" $ do
        it "starts at zero without a phase offset" $ do
            wave <- rightValue $ newWave 5.0 100.0
            evaluate wave 0.0 `shouldBeCloseTo` 0.0
        it "reaches the peak at a quarter period" $ do
            wave <- rightValue $ newWave 1.0 1.0
            evaluate wave 0.25 `shouldBeCloseTo` 1.0
        it "returns to zero at half a period" $ do
            wave <- rightValue $ newWave 1.0 1.0
            evaluate wave 0.5 `shouldBeCloseTo` 0.0
        it "reaches the trough at three quarters of a period" $ do
            wave <- rightValue $ newWave 1.0 1.0
            evaluate wave 0.75 `shouldBeCloseTo` (-1.0)
        it "is periodic" $ do
            wave <- rightValue $ newWaveWithPhase 2.5 3.0 0.7
            let time = 0.137
            evaluate wave time `shouldBeCloseTo` evaluate wave (time + period wave)
        it "rejects nonfinite time before the zero-amplitude shortcut" $ do
            wave <- rightValue $ newWave 0.0 1.0
            Exception.evaluate (Wave.evaluate wave (0.0 / 0.0))
                `shouldThrow` anyErrorCall
            Exception.evaluate (Wave.evaluate wave (1.0 / 0.0))
                `shouldThrow` anyErrorCall
        it "keeps canonical extreme inputs finite and bounded" $ do
            zero <- rightValue $ newWaveWithPhase 0.0 1e300 Trig.halfPi
            let zeroResult = evaluate zero maxFinite
            zeroResult `shouldBe` 0.0
            isNegativeZero zeroResult `shouldBe` False

            extreme <- rightValue $ newWaveWithPhase maxFinite 1e300 Trig.halfPi
            let result = evaluate extreme maxFinite
            isNaN result `shouldBe` False
            isInfinite result `shouldBe` False
            abs result `shouldSatisfy` (<= maxFinite)
        it "supports a minimum-subnormal frequency with an infinite period" $ do
            wave <- rightValue $ newWaveWithPhase 1.0 (encodeFloat 1 (-1074)) maxFinite
            isInfinite (period wave) `shouldBe` True
            let result = evaluate wave maxFinite
            isNaN result `shouldBe` False
            isInfinite result `shouldBe` False
            abs result `shouldSatisfy` (<= 1.0)

    describe "phase offsets" $ do
        it "starts at the peak with phase pi over two" $ do
            wave <- rightValue $ newWaveWithPhase 1.0 1.0 Trig.halfPi
            evaluate wave 0.0 `shouldBeCloseTo` 1.0
        it "starts at the trough with phase three pi over two" $ do
            wave <- rightValue $ newWaveWithPhase 1.0 1.0 (3.0 * Trig.halfPi)
            evaluate wave 0.0 `shouldBeCloseTo` (-1.0)

    describe "zero amplitude" $
        it "is always zero" $ do
            wave <- rightValue $ newWaveWithPhase 0.0 1.0 Trig.halfPi
            mapM_ (\time -> evaluate wave time `shouldBeCloseTo` 0.0)
                [0.0, 0.25, 0.5, 0.75, 1.0]
