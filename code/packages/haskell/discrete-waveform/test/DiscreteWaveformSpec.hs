module DiscreteWaveformSpec (spec) where

import Test.Hspec
import DiscreteWaveform

spec :: Spec
spec = do
    describe "Waveform" $ do
        it "samples correctly" $ do
            let w0 = newWaveform
            let w1 = addSample w0 0 1
            let w2 = addSample w1 5 0
            sample w2 2 `shouldBe` Just 1
            sample w2 6 `shouldBe` Just 0
            
    describe "Generator" $ do
        it "generates square wave" $ do
            let sq = newSquareWave 10
            let wf = generate sq 0 20 1
            sample wf 2 `shouldBe` Just 1
            sample wf 6 `shouldBe` Just 0
