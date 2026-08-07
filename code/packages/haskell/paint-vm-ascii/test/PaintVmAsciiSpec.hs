module PaintVmAsciiSpec (spec) where

import CodingAdventures.PaintInstructions
  ( PaintInstruction
  , PaintScene (..)
  , emptyScene
  , makePath
  , makeRect
  )
import CodingAdventures.PaintVmAscii
import Test.Hspec

spec :: Spec
spec = do
  describe "metadata" $ do
    it "reports the shared package version" $
      version `shouldBe` "0.1.0"

    it "uses the shared default cell scaling" $
      defaultAsciiOptions `shouldBe` AsciiOptions 8 16

  describe "render" $ do
    it "renders a filled rectangle inclusively" $ do
      let scene = withInstructions (emptyScene 4 3 "#ffffff")
            [makeRect 0 0 2 1 "#000000"]
      render scene (AsciiOptions 1 1) `shouldBe` Right "███\n███"

    it "uses painter order while clipping rectangles to the buffer" $ do
      let scene = withInstructions (emptyScene 3 2 "transparent")
            [ makeRect (-2) (-2) 3 3 "red"
            , makeRect 2 1 4 4 "blue"
            ]
      render scene (AsciiOptions 1 1) `shouldBe` Right "██\n███"

    it "skips empty and transparent fills after trimming whitespace" $ do
      let scene = withInstructions (emptyScene 3 2 "transparent")
            [ makeRect 0 0 2 1 ""
            , makeRect 0 0 2 1 " transparent "
            , makeRect 0 0 2 1 "none"
            ]
      render scene (AsciiOptions 1 1) `shouldBe` Right ""

    it "maps coordinates through the default scale" $ do
      let scene = withInstructions (emptyScene 16 32 "transparent")
            [makeRect 8 16 0 0 "black"]
      renderDefault scene `shouldBe` Right "\n █"

    it "uses nearest-even rounding at half-cell boundaries" $ do
      let scene = withInstructions (emptyScene 4 1 "transparent")
            [makeRect 0.5 0 1 0 "black"]
      render scene (AsciiOptions 1 1) `shouldBe` Right "███"

    it "renders a zero-sized scene as empty text" $
      renderDefault (emptyScene 0 0 "transparent") `shouldBe` Right ""

    it "rejects paths instead of returning an incomplete rendering" $ do
      let scene = withInstructions (emptyScene 10 10 "transparent")
            [makePath [] "black"]
      renderDefault scene `shouldBe` Left (UnsupportedInstruction "path")

    it "rejects non-positive horizontal scales" $ do
      let scene = emptyScene 1 1 "transparent"
      render scene (AsciiOptions 0 1) `shouldBe` Left (InvalidScaleX 0)
      render scene (AsciiOptions (-1) 1) `shouldBe` Left (InvalidScaleX (-1))

    it "rejects non-positive vertical scales" $ do
      let scene = emptyScene 1 1 "transparent"
      render scene (AsciiOptions 1 0) `shouldBe` Left (InvalidScaleY 0)
      render scene (AsciiOptions 1 (-1)) `shouldBe` Left (InvalidScaleY (-1))

    it "rejects negative and non-finite scene dimensions" $ do
      renderDefault (emptyScene (-1) 1 "transparent")
        `shouldBe` Left (InvalidSceneDimensions (-1) 1)
      renderDefault (emptyScene (0 / 0) 1 "transparent")
        `shouldSatisfy` isInvalidDimensions
      renderDefault (emptyScene (1 / 0) 1 "transparent")
        `shouldBe` Left (InvalidSceneDimensions (1 / 0) 1)

    it "rejects invalid rectangle geometry" $ do
      let negativeSize = withInstructions (emptyScene 2 2 "transparent")
            [makeRect 0 0 (-1) 1 "black"]
          infiniteCoordinate = withInstructions (emptyScene 2 2 "transparent")
            [makeRect (1 / 0) 0 1 1 "black"]
      renderDefault negativeSize
        `shouldBe` Left (InvalidRectangleGeometry 0 0 (-1) 1)
      renderDefault infiniteCoordinate
        `shouldBe` Left (InvalidRectangleGeometry (1 / 0) 0 1 1)

withInstructions :: PaintScene -> [PaintInstruction] -> PaintScene
withInstructions scene instructions = scene { psInstructions = instructions }

isInvalidDimensions :: Either PaintVmAsciiError String -> Bool
isInvalidDimensions (Left (InvalidSceneDimensions width height)) = isNaN width && height == 1
isInvalidDimensions _ = False
