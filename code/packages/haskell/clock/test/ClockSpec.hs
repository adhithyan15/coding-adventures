module ClockSpec (spec) where

import Test.Hspec
import Clock

spec :: Spec
spec = do
    describe "Clock" $ do
        it "ticks and updates values correctly" $ do
            let clk0 = newClock
            let (clk1, v1) = tick clk0
            v1 `shouldBe` 1
            cycleCount clk1 `shouldBe` 1
            
            let (clk2, v2) = tick clk1
            v2 `shouldBe` 0
            cycleCount clk2 `shouldBe` 1
            
        it "divider ticks properly" $ do
            let div0 = newClockDivider 2
            let (div1, v1) = tickDivider div0
            let (div2, v2) = tickDivider div1
            -- Need full cycles to trigger divider
            -- 1 base flip to 1 -> counter 1
            -- 2 base flips to 0 -> nothing
            -- 3 base flip to 1 -> counter 2 -> div flip to 1
            cdValue div1 `shouldBe` 0
