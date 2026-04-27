module PowerSupplySpec (spec) where

import Test.Hspec
import PowerSupply

spec :: Spec
spec = do
    describe "PowerSupply" $ do
        it "outputs voltage correctly" $ do
            let ps = newPowerSupply 5.0
            readVoltage ps `shouldBe` 5.0
