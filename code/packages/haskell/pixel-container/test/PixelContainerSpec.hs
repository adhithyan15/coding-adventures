module PixelContainerSpec (spec) where

import Test.Hspec
import PixelContainer
import qualified Data.ByteString as BS

spec :: Spec
spec = do
    describe "createPixelContainer" $ do
        it "stores the requested width" $
            pcWidth (createPixelContainer 4 3) `shouldBe` 4
        it "stores the requested height" $
            pcHeight (createPixelContainer 4 3) `shouldBe` 3
        it "allocates width * height * 4 bytes" $
            BS.length (pcPixels (createPixelContainer 4 3)) `shouldBe` 48
        it "initialises every byte to zero" $
            BS.all (== 0) (pcPixels (createPixelContainer 4 3)) `shouldBe` True
        it "handles a 1x1 container" $ do
            let pc = createPixelContainer 1 1
            pcWidth pc `shouldBe` 1
            pcHeight pc `shouldBe` 1
            BS.length (pcPixels pc) `shouldBe` 4
        it "handles a 0x0 container" $ do
            let pc = createPixelContainer 0 0
            pcWidth pc `shouldBe` 0
            pcHeight pc `shouldBe` 0
            BS.length (pcPixels pc) `shouldBe` 0
        it "handles a 0 x h container" $
            BS.length (pcPixels (createPixelContainer 0 5)) `shouldBe` 0
        it "handles a w x 0 container" $
            BS.length (pcPixels (createPixelContainer 5 0)) `shouldBe` 0

    describe "pixelAt" $ do
        it "returns (0,0,0,0) for a freshly created pixel" $
            pixelAt (createPixelContainer 2 2) 0 0 `shouldBe` (0, 0, 0, 0)
        it "returns (0,0,0,0) for negative x" $
            pixelAt (createPixelContainer 2 2) (-1) 0 `shouldBe` (0, 0, 0, 0)
        it "returns (0,0,0,0) for negative y" $
            pixelAt (createPixelContainer 2 2) 0 (-1) `shouldBe` (0, 0, 0, 0)
        it "returns (0,0,0,0) for x past right edge" $
            pixelAt (createPixelContainer 2 2) 2 0 `shouldBe` (0, 0, 0, 0)
        it "returns (0,0,0,0) for y past bottom edge" $
            pixelAt (createPixelContainer 2 2) 0 2 `shouldBe` (0, 0, 0, 0)

    describe "setPixel" $ do
        it "round-trips via pixelAt" $ do
            let pc  = createPixelContainer 3 3
                pc' = setPixel pc 1 1 10 20 30 40
            pixelAt pc' 1 1 `shouldBe` (10, 20, 30, 40)
        it "does not disturb neighbouring pixels" $ do
            let pc  = createPixelContainer 3 3
                pc' = setPixel pc 1 1 10 20 30 40
            pixelAt pc' 0 1 `shouldBe` (0, 0, 0, 0)
            pixelAt pc' 2 1 `shouldBe` (0, 0, 0, 0)
            pixelAt pc' 1 0 `shouldBe` (0, 0, 0, 0)
            pixelAt pc' 1 2 `shouldBe` (0, 0, 0, 0)
        it "is a no-op for negative x" $ do
            let pc  = createPixelContainer 2 2
                pc' = setPixel pc (-1) 0 1 2 3 4
            pcPixels pc' `shouldBe` pcPixels pc
        it "is a no-op for negative y" $ do
            let pc  = createPixelContainer 2 2
                pc' = setPixel pc 0 (-1) 1 2 3 4
            pcPixels pc' `shouldBe` pcPixels pc
        it "is a no-op for x past right edge" $ do
            let pc  = createPixelContainer 2 2
                pc' = setPixel pc 2 0 1 2 3 4
            pcPixels pc' `shouldBe` pcPixels pc
        it "is a no-op for y past bottom edge" $ do
            let pc  = createPixelContainer 2 2
                pc' = setPixel pc 0 2 1 2 3 4
            pcPixels pc' `shouldBe` pcPixels pc
        it "supports multiple round-trip writes" $ do
            let pc0 = createPixelContainer 3 2
                pc1 = setPixel pc0 0 0 11 12 13 14
                pc2 = setPixel pc1 2 1 21 22 23 24
            pixelAt pc2 0 0 `shouldBe` (11, 12, 13, 14)
            pixelAt pc2 2 1 `shouldBe` (21, 22, 23, 24)
            pixelAt pc2 1 0 `shouldBe` (0, 0, 0, 0)
        it "overwrites an existing pixel" $ do
            let pc0 = createPixelContainer 2 2
                pc1 = setPixel pc0 1 1 10 20 30 40
                pc2 = setPixel pc1 1 1 99 99 99 99
            pixelAt pc2 1 1 `shouldBe` (99, 99, 99, 99)

    describe "fillPixels" $ do
        it "sets every pixel to the given colour" $ do
            let pc  = createPixelContainer 3 2
                pc' = fillPixels pc 7 8 9 10
            mapM_ (\(x, y) -> pixelAt pc' x y `shouldBe` (7, 8, 9, 10))
                  [(x, y) | y <- [0..1], x <- [0..2]]
        it "preserves container dimensions" $ do
            let pc' = fillPixels (createPixelContainer 4 5) 1 2 3 4
            pcWidth  pc' `shouldBe` 4
            pcHeight pc' `shouldBe` 5
        it "emits width*height*4 bytes" $
            BS.length (pcPixels (fillPixels (createPixelContainer 4 5) 1 2 3 4))
                `shouldBe` 80
        it "is idempotent when applied twice with the same colour" $ do
            let pc1 = fillPixels (createPixelContainer 3 3) 1 2 3 4
                pc2 = fillPixels pc1 1 2 3 4
            pcPixels pc1 `shouldBe` pcPixels pc2
        it "handles fill on a 0x0 container" $
            pcPixels (fillPixels (createPixelContainer 0 0) 1 2 3 4)
                `shouldBe` BS.empty
