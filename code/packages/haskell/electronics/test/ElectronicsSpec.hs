module ElectronicsSpec (spec) where

import Test.Hspec
import Electronics

spec :: Spec
spec = do
    describe "RC Filter" $ do
        it "steps filter correctly" $ do
            let flt0 = newRCFilter 1000 0.001
            let flt1 = stepFilter flt0 5.0 0.5
            nodeVoltage (rcNode flt1) `shouldBe` 2.5
