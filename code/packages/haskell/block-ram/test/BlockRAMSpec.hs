module BlockRAMSpec (spec) where

import Test.Hspec
import BlockRAM
import LogicGates

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

    describe "Hardening (LGE-HS-DEPS)" $ do
        it "newBRAM rejects zero/negative width and total without DivideByZero" $ do
            newBRAM 1024 0 `shouldBe` Left (InvalidShape "Width must be >= 1")
            newBRAM 1024 (-4) `shouldBe` Left (InvalidShape "Width must be >= 1")
            newBRAM 0 8 `shouldBe` Left (InvalidShape "Total bits must be >= 1")
            either Just (const Nothing) (newBRAM 4 8) `shouldBe` Just (InvalidShape "Depth must be >= 1")
        it "caps RAM sizes" $ do
            either Just (const Nothing) (newSinglePortRAM 2000000 8 ReadFirst) `shouldBe` Just (InvalidShape "RAM exceeds maxRAMBits")
            either Just (const Nothing) (newDualPortRAM maxBound maxBound) `shouldBe` Just (InvalidShape "Width exceeds maxDataWidth")
            either Just (const Nothing) (newBRAM (2^(40::Int)) 8) `shouldBe` Just (InvalidShape "RAM exceeds maxRAMBits")
            either Just (const Nothing) (newSRAMArray 1 70000) `shouldBe` Just (InvalidShape "Width exceeds maxDataWidth")
            either (const False) (const True) (newBRAM 1024 8) `shouldBe` True
        it "single-port validates clock, writeEnable, dataIn" $ do
            let Right ram = newSinglePortRAM 4 1 ReadFirst
            fmap fst (tickSinglePortRAM ram 1 0 [5] 1) `shouldBe` Left (InvalidBit "dataIn" 5)
            fmap fst (tickSinglePortRAM ram 1 0 [1] 2) `shouldBe` Left (InvalidBit "writeEnable" 2)
            fmap fst (tickSinglePortRAM ram 2 0 [1] 1) `shouldBe` Left (InvalidBit "clock" 2)
            fmap fst (tickSinglePortRAM ram 0 0 [9] 0) `shouldBe` Left (InvalidBit "dataIn" 9)
            fmap fst (tickSinglePortRAM ram 1 0 [1,1] 1) `shouldBe` Left (InvalidShape "Data length mismatch")
        it "dual-port validates both ports" $ do
            let Right ram = newDualPortRAM 4 1
            let t c a da wa b db wb = fmap (\(_,x,y) -> (x,y)) (tickDualPortRAM ram c a da wa b db wb)
            t 3 0 [0] 0 1 [0] 0 `shouldBe` Left (InvalidBit "clock" 3)
            t 1 0 [0] 2 1 [0] 0 `shouldBe` Left (InvalidBit "weA" 2)
            t 1 0 [0] 0 1 [0] 2 `shouldBe` Left (InvalidBit "weB" 2)
            t 1 0 [5] 0 1 [0] 0 `shouldBe` Left (InvalidBit "dataInA" 5)
            t 1 0 [0] 0 1 [7] 0 `shouldBe` Left (InvalidBit "dataInB" 7)
            t 1 9 [0] 0 0 [0] 0 `shouldBe` Left (InvalidShape "Address bounds")
        it "writeSRAMArray validates data (defence in depth)" $ do
            let Right arr = newSRAMArray 2 1
            fmap (const ()) (writeSRAMArray arr 0 [4]) `shouldBe` Left (InvalidBit "dataIn" 4)
            fmap (const ()) (writeSRAMArray arr 5 [1]) `shouldBe` Left (InvalidShape "Row index out of bounds")
            fmap (const ()) (readSRAMArray arr (-1)) `shouldBe` Left (InvalidShape "Row index out of bounds")
