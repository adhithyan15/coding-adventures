module ImageGeometricTransformsSpec (spec) where

import Test.Hspec
import PixelContainer
import ImageGeometricTransforms
import Data.Word (Word8)

-- | Convenience: a small test image with each pixel uniquely colored
-- by its coordinates so we can check transforms by inspecting pixels.
mkGrad :: Int -> Int -> PixelContainer
mkGrad w h =
    foldl step (createPixelContainer w h) [(x, y) | y <- [0..h-1], x <- [0..w-1]]
  where
    step pc (x, y) =
        let r = fromIntegral (x * 20) :: Word8
            g = fromIntegral (y * 20) :: Word8
            b = 0 :: Word8
            a = 255 :: Word8
        in setPixel pc x y r g b a

-- | Tolerance-based pixel comparison for floating-point paths.
near :: Word8 -> Word8 -> Int -> Bool
near a b tol = abs (fromIntegral a - fromIntegral b :: Int) <= tol

spec :: Spec
spec = do
    describe "flipHorizontal" $ do
        it "preserves dimensions" $ do
            let pc = flipHorizontal (mkGrad 4 3)
            pcWidth pc  `shouldBe` 4
            pcHeight pc `shouldBe` 3
        it "mirrors columns" $ do
            let src = mkGrad 4 3
                pc  = flipHorizontal src
            pixelAt pc 0 0 `shouldBe` pixelAt src 3 0
            pixelAt pc 3 2 `shouldBe` pixelAt src 0 2
        it "is an involution (twice = identity)" $ do
            let src = mkGrad 4 3
                pc  = flipHorizontal (flipHorizontal src)
            pcPixels pc `shouldBe` pcPixels src

    describe "flipVertical" $ do
        it "preserves dimensions" $ do
            let pc = flipVertical (mkGrad 4 3)
            pcWidth pc  `shouldBe` 4
            pcHeight pc `shouldBe` 3
        it "mirrors rows" $ do
            let src = mkGrad 4 3
                pc  = flipVertical src
            pixelAt pc 0 0 `shouldBe` pixelAt src 0 2
            pixelAt pc 3 2 `shouldBe` pixelAt src 3 0
        it "is an involution" $ do
            let src = mkGrad 4 3
                pc  = flipVertical (flipVertical src)
            pcPixels pc `shouldBe` pcPixels src

    describe "rotate90CW" $ do
        it "swaps dimensions" $ do
            let pc = rotate90CW (mkGrad 4 3)
            pcWidth pc  `shouldBe` 3
            pcHeight pc `shouldBe` 4
        it "places (0,0) at the top-right of the source" $ do
            let src = mkGrad 4 3
                pc  = rotate90CW src
            pixelAt pc 0 0 `shouldBe` pixelAt src 0 2
        it "four applications is identity" $ do
            let src = mkGrad 4 3
                pc  = rotate90CW (rotate90CW (rotate90CW (rotate90CW src)))
            pcPixels pc `shouldBe` pcPixels src

    describe "rotate90CCW" $ do
        it "swaps dimensions" $ do
            let pc = rotate90CCW (mkGrad 4 3)
            pcWidth pc  `shouldBe` 3
            pcHeight pc `shouldBe` 4
        it "CW then CCW is identity" $ do
            let src = mkGrad 4 3
                pc  = rotate90CCW (rotate90CW src)
            pcPixels pc `shouldBe` pcPixels src

    describe "rotate180" $ do
        it "preserves dimensions" $ do
            let pc = rotate180 (mkGrad 4 3)
            pcWidth pc  `shouldBe` 4
            pcHeight pc `shouldBe` 3
        it "equals double rotate90CW" $ do
            let src = mkGrad 4 3
                a   = rotate180 src
                b   = rotate90CW (rotate90CW src)
            pcPixels a `shouldBe` pcPixels b
        it "is an involution" $ do
            let src = mkGrad 4 3
                pc  = rotate180 (rotate180 src)
            pcPixels pc `shouldBe` pcPixels src

    describe "crop" $ do
        it "extracts correct region" $ do
            let src = mkGrad 4 4
                pc  = crop src 1 1 2 2
            pcWidth pc  `shouldBe` 2
            pcHeight pc `shouldBe` 2
            pixelAt pc 0 0 `shouldBe` pixelAt src 1 1
            pixelAt pc 1 1 `shouldBe` pixelAt src 2 2
        it "fills out-of-bounds with (0,0,0,0)" $ do
            let src = mkGrad 2 2
                pc  = crop src 0 0 4 4
            pixelAt pc 3 3 `shouldBe` (0, 0, 0, 0)

    describe "scale (nearest)" $ do
        it "doubling preserves visual content approximately" $ do
            let src = mkGrad 4 4
                pc  = scale src 8 8 Nearest Zero
            pcWidth pc  `shouldBe` 8
            pcHeight pc `shouldBe` 8
        it "identity scale is identity" $ do
            let src = mkGrad 4 4
                pc  = scale src 4 4 Nearest Zero
            -- Nearest identity: each output pixel equals same source pixel.
            pixelAt pc 0 0 `shouldBe` pixelAt src 0 0
            pixelAt pc 3 3 `shouldBe` pixelAt src 3 3

    describe "scale (bilinear)" $ do
        it "approximately preserves uniform colour" $ do
            let src = fillPixels (createPixelContainer 4 4) 100 150 200 255
                pc  = scale src 8 8 Bilinear Replicate
                (r, g, b, _) = pixelAt pc 4 4
            near r 100 3 `shouldBe` True
            near g 150 3 `shouldBe` True
            near b 200 3 `shouldBe` True

    describe "scale (bicubic)" $ do
        it "preserves uniform colour" $ do
            let src = fillPixels (createPixelContainer 4 4) 50 100 150 255
                pc  = scale src 8 8 Bicubic Replicate
                (r, g, b, _) = pixelAt pc 4 4
            near r 50  4 `shouldBe` True
            near g 100 4 `shouldBe` True
            near b 150 4 `shouldBe` True

    describe "rotate (arbitrary)" $ do
        it "0 degrees Crop is approximately identity" $ do
            let src = fillPixels (createPixelContainer 4 4) 200 100 50 255
                pc  = rotate src 0 Crop Nearest Replicate
                (r, g, b, _) = pixelAt pc 2 2
            near r 200 1 `shouldBe` True
            near g 100 1 `shouldBe` True
            near b 50  1 `shouldBe` True
        it "90 degrees Fit produces a canvas of swapped dimensions" $ do
            let src = mkGrad 4 2
                pc  = rotate src 90 Fit Nearest Zero
            pcWidth pc  `shouldBe` 2
            pcHeight pc `shouldBe` 4
        it "Crop keeps canvas size" $ do
            let pc = rotate (mkGrad 4 4) 45 Crop Bilinear Zero
            pcWidth pc  `shouldBe` 4
            pcHeight pc `shouldBe` 4

    describe "translate" $ do
        it "integer shift moves pixels" $ do
            let src = mkGrad 4 4
                pc  = translate src 1 0 Nearest Zero
            pixelAt pc 1 0 `shouldBe` pixelAt src 0 0
        it "negative shift with Zero OOB fills zeros" $ do
            let src = fillPixels (createPixelContainer 4 4) 200 100 50 255
                pc  = translate src 2 0 Nearest Zero
            pixelAt pc 0 0 `shouldBe` (0, 0, 0, 0)
        it "Replicate OOB extends edge" $ do
            let src = fillPixels (createPixelContainer 4 4) 200 100 50 255
                pc  = translate src 2 0 Nearest Replicate
            pixelAt pc 0 0 `shouldBe` (200, 100, 50, 255)

    describe "affine" $ do
        it "identity matrix is (approximately) identity" $ do
            let src = fillPixels (createPixelContainer 4 4) 123 45 67 255
                m   = [[1,0,0],[0,1,0]]
                pc  = affine src m Nearest Replicate
            pixelAt pc 2 2 `shouldBe` (123, 45, 67, 255)
        it "2x scale matrix zooms in" $ do
            let src = mkGrad 4 4
                -- Forward map (x,y) -> (2x, 2y).  Inverse samples at (x'/2, y'/2).
                m   = [[2,0,0],[0,2,0]]
                pc  = affine src m Nearest Replicate
            -- Pixel (2, 2) of the output should read the source at
            -- u = (x - c) * ia = (2 - 0) * 0.5 = 1, v = 1.
            pixelAt pc 2 2 `shouldBe` pixelAt src 1 1

    describe "perspectiveWarp" $ do
        it "identity homography is (approximately) identity" $ do
            let src = fillPixels (createPixelContainer 4 4) 30 60 90 255
                m   = [[1,0,0],[0,1,0],[0,0,1]]
                pc  = perspectiveWarp src m Nearest Replicate
                (r, g, b, _) = pixelAt pc 2 2
            (r, g, b) `shouldBe` (30, 60, 90)
        it "pure scaling homography matches affine scaling" $ do
            let src = mkGrad 4 4
                m   = [[2,0,0],[0,2,0],[0,0,1]]
                pc  = perspectiveWarp src m Nearest Replicate
            pixelAt pc 2 2 `shouldBe` pixelAt src 1 1

    describe "Out-of-bounds policies" $ do
        it "Zero returns transparent black when crossing boundary" $ do
            let src = fillPixels (createPixelContainer 4 4) 200 100 50 255
                pc  = translate src 10 10 Nearest Zero
            pixelAt pc 0 0 `shouldBe` (0, 0, 0, 0)
        it "Replicate clamps to edge pixel" $ do
            let src = fillPixels (createPixelContainer 4 4) 200 100 50 255
                pc  = translate src 10 10 Nearest Replicate
            pixelAt pc 0 0 `shouldBe` (200, 100, 50, 255)
        it "Wrap tiles the image" $ do
            let src = fillPixels (createPixelContainer 4 4) 200 100 50 255
                pc  = translate src 4 0 Nearest Wrap
            pixelAt pc 0 0 `shouldBe` (200, 100, 50, 255)
        it "Reflect produces in-bounds samples" $ do
            let src = mkGrad 4 4
                pc  = translate src (-1) 0 Nearest Reflect
                (_,_,_,a) = pixelAt pc 0 0
            -- Should be opaque because sample was in-bounds.
            a `shouldBe` 255
