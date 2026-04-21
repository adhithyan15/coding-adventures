module ImagePointOpsSpec (spec) where

import Test.Hspec
import PixelContainer
import ImagePointOps
import Data.Word (Word8)

-- | Build a single-pixel image in one line.
onePixel :: Word8 -> Word8 -> Word8 -> Word8 -> PixelContainer
onePixel r g b a = setPixel (createPixelContainer 1 1) 0 0 r g b a

-- | Tolerance for comparing round-tripped byte values that go through
-- floating-point arithmetic.
near :: Word8 -> Word8 -> Int -> Bool
near a b tol = abs (fromIntegral a - fromIntegral b :: Int) <= tol

spec :: Spec
spec = do
    describe "invert" $ do
        it "inverts pure white to black" $
            pixelAt (invert (onePixel 255 255 255 255)) 0 0
                `shouldBe` (0, 0, 0, 255)
        it "inverts pure black to white" $
            pixelAt (invert (onePixel 0 0 0 128)) 0 0
                `shouldBe` (255, 255, 255, 128)
        it "preserves alpha" $
            let (_, _, _, a) = pixelAt (invert (onePixel 10 20 30 200)) 0 0
            in a `shouldBe` 200

    describe "threshold" $ do
        it "sends darks to black" $
            pixelAt (threshold (onePixel 10 10 10 255) 128) 0 0
                `shouldBe` (0, 0, 0, 255)
        it "sends brights to white" $
            pixelAt (threshold (onePixel 200 200 200 255) 128) 0 0
                `shouldBe` (255, 255, 255, 255)

    describe "thresholdLuminance" $ do
        it "treats bright green as white" $
            pixelAt (thresholdLuminance (onePixel 0 255 0 255) 128) 0 0
                `shouldBe` (255, 255, 255, 255)
        it "treats deep blue as black" $
            pixelAt (thresholdLuminance (onePixel 0 0 255 255) 128) 0 0
                `shouldBe` (0, 0, 0, 255)

    describe "posterize" $ do
        it "snaps values with levels=2 to 0 or 255" $ do
            let (r, _, _, _) = pixelAt (posterize (onePixel 10 10 10 255) 2) 0 0
            r `shouldBe` 0
            let (r', _, _, _) = pixelAt (posterize (onePixel 200 200 200 255) 2) 0 0
            r' `shouldBe` 255
        it "clamps levels<2 to 2" $ do
            let (r, _, _, _) = pixelAt (posterize (onePixel 200 200 200 255) 1) 0 0
            r `shouldBe` 255

    describe "swapRgbBgr" $ do
        it "swaps R and B" $
            pixelAt (swapRgbBgr (onePixel 10 20 30 40)) 0 0
                `shouldBe` (30, 20, 10, 40)

    describe "extractChannel" $ do
        it "extracts red as greyscale" $
            pixelAt (extractChannel (onePixel 100 50 25 255) 0) 0 0
                `shouldBe` (100, 100, 100, 255)
        it "extracts green" $
            pixelAt (extractChannel (onePixel 100 50 25 255) 1) 0 0
                `shouldBe` (50, 50, 50, 255)
        it "extracts blue" $
            pixelAt (extractChannel (onePixel 100 50 25 255) 2) 0 0
                `shouldBe` (25, 25, 25, 255)
        it "extracts alpha" $
            pixelAt (extractChannel (onePixel 100 50 25 77) 3) 0 0
                `shouldBe` (77, 77, 77, 255)

    describe "brightness" $ do
        it "adds positive delta and clamps" $
            pixelAt (brightness (onePixel 200 200 200 255) 100) 0 0
                `shouldBe` (255, 255, 255, 255)
        it "subtracts and clamps to zero" $
            pixelAt (brightness (onePixel 10 10 10 255) (-100)) 0 0
                `shouldBe` (0, 0, 0, 255)
        it "is identity at delta=0" $
            pixelAt (brightness (onePixel 10 20 30 40) 0) 0 0
                `shouldBe` (10, 20, 30, 40)

    describe "contrast" $ do
        it "is approximately identity at factor=0" $ do
            let (r, _, _, _) = pixelAt (contrast (onePixel 128 128 128 255) 0) 0 0
            near r 128 1 `shouldBe` True

    describe "gamma" $ do
        it "leaves black fixed" $
            pixelAt (gamma (onePixel 0 0 0 255) 2.2) 0 0
                `shouldBe` (0, 0, 0, 255)
        it "leaves white fixed" $
            pixelAt (gamma (onePixel 255 255 255 255) 2.2) 0 0
                `shouldBe` (255, 255, 255, 255)

    describe "exposure" $ do
        it "doubles brightness at +1 stop" $ do
            let (r, _, _, _) = pixelAt (exposure (onePixel 64 64 64 255) 1.0) 0 0
            r > 64 `shouldBe` True
        it "halves brightness at -1 stop" $ do
            let (r, _, _, _) = pixelAt (exposure (onePixel 200 200 200 255) (-1.0)) 0 0
            r < 200 `shouldBe` True

    describe "greyscale" $ do
        it "produces equal R=G=B" $ do
            let (r, g, b, _) = pixelAt (greyscale (onePixel 200 100 50 255) Rec709) 0 0
            r `shouldBe` g
            g `shouldBe` b
        it "average method treats channels equally" $ do
            let (r1, _, _, _) = pixelAt (greyscale (onePixel 255 0 0 255) Average) 0 0
                (r2, _, _, _) = pixelAt (greyscale (onePixel 0 255 0 255) Average) 0 0
            r1 `shouldBe` r2

    describe "sepia" $ do
        it "tints white to a warm tone" $ do
            let (r, g, b, _) = pixelAt (sepia (onePixel 255 255 255 255)) 0 0
            r >= g `shouldBe` True
            g >= b `shouldBe` True
        it "leaves black black" $
            pixelAt (sepia (onePixel 0 0 0 255)) 0 0
                `shouldBe` (0, 0, 0, 255)

    describe "colourMatrix" $ do
        it "identity matrix is a round-trip" $ do
            let m = [[1,0,0],[0,1,0],[0,0,1]]
                (r, g, b, _) = pixelAt (colourMatrix (onePixel 100 150 200 255) m) 0 0
            near r 100 1 `shouldBe` True
            near g 150 1 `shouldBe` True
            near b 200 1 `shouldBe` True

    describe "saturate" $ do
        it "factor=0 produces grey" $ do
            let (r, g, b, _) = pixelAt (saturate (onePixel 200 50 25 255) 0) 0 0
            r `shouldBe` g
            g `shouldBe` b
        it "factor=1 is approximately identity" $ do
            let (r, _, _, _) = pixelAt (saturate (onePixel 200 50 25 255) 1) 0 0
            near r 200 2 `shouldBe` True

    describe "hueRotate" $ do
        it "0 degrees is approximately identity" $ do
            let (r, g, b, _) = pixelAt (hueRotate (onePixel 200 50 25 255) 0) 0 0
            near r 200 2 `shouldBe` True
            near g 50  2 `shouldBe` True
            near b 25  2 `shouldBe` True
        it "360 degrees is approximately identity" $ do
            let (r, _, _, _) = pixelAt (hueRotate (onePixel 200 50 25 255) 360) 0 0
            near r 200 2 `shouldBe` True
        it "leaves greyscale unchanged" $ do
            let (r, g, b, _) = pixelAt (hueRotate (onePixel 128 128 128 255) 90) 0 0
            r `shouldBe` g
            g `shouldBe` b

    describe "srgbToLinearImage / linearToSrgbImage" $ do
        it "round-trips approximately" $ do
            let pc   = onePixel 128 200 50 255
                pc'  = linearToSrgbImage (srgbToLinearImage pc)
                (r, g, b, _) = pixelAt pc' 0 0
            near r 128 3 `shouldBe` True
            near g 200 3 `shouldBe` True
            near b 50  3 `shouldBe` True

    describe "applyLut1dU8" $ do
        it "identity LUT is a no-op" $ do
            let ident = [fromIntegral i :: Word8 | i <- [0..255 :: Int]]
                (r, g, b, _) = pixelAt
                    (applyLut1dU8 (onePixel 10 20 30 40) ident ident ident) 0 0
            (r, g, b) `shouldBe` (10, 20, 30)
        it "constant LUT overrides the channel" $ do
            let zeroes = replicate 256 (0 :: Word8)
                (r, _, _, _) = pixelAt
                    (applyLut1dU8 (onePixel 200 200 200 255) zeroes zeroes zeroes) 0 0
            r `shouldBe` 0

    describe "buildLut1dU8" $ do
        it "identity function yields identity LUT" $ do
            let lut = buildLut1dU8 id
            length lut `shouldBe` 256
            -- Index 0 maps to 0; index 255 maps to 255.
            head lut `shouldBe` 0
            last lut `shouldBe` 255

    describe "buildGammaLut" $ do
        it "gamma=1 is approximately identity" $ do
            let lut = buildGammaLut 1.0
            length lut `shouldBe` 256
            near (head lut) 0   1 `shouldBe` True
            near (last lut) 255 1 `shouldBe` True
