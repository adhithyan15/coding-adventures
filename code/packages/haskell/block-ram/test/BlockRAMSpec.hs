module BlockRAMSpec (spec) where

import Test.Hspec
import BlockRAM

spec :: Spec
spec = do
    describe "SRAM" $ do
        it "reads and writes correctly" $ do
            let cell1 = writeSRAMCell newSRAMCell 1 1
            readSRAMCell cell1 1 `shouldBe` Just 1
    describe "RAM" $ do
        it "write and reads dual port properly" $ do
            let Right ram0 = newDualPortRAM 16 8
            let Right (ram1, _, _) = tickDualPortRAM ram0 1 0 [1,1,1,1,0,0,0,0] 1 1 [0,0,0,0,1,1,1,1] 1
            let Right (_, outA, outB) = tickDualPortRAM ram1 1 0 [0,0,0,0,0,0,0,0] 0 1 [0,0,0,0,0,0,0,0] 0
            outA `shouldBe` [1,1,1,1,0,0,0,0]
            outB `shouldBe` [0,0,0,0,1,1,1,1]
