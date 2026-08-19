module PaintVmAsciiSpec (spec) where

import CodingAdventures.PaintInstructions
  ( PaintGlyphPlacement (..)
  , PaintInstruction (..)
  , PaintScene (..)
  , emptyScene
  , makeClip
  , makeGlyphRun
  , makeGroup
  , makeLayer
  , makeLine
  , makePath
  , makeRect
  )
import CodingAdventures.PaintVmAscii
import Test.Hspec

spec :: Spec
spec = do
  describe "metadata" $ do
    it "reports the shared package version" $
      version `shouldBe` "0.2.0"

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

    it "rejects a rectangle whose individually-finite x+w overflows to infinity" $ do
      let hugeX = 1.7e308 :: Double
          hugeW = 1.0e308 :: Double
          scene = withInstructions (emptyScene 2 2 "transparent")
            [makeRect hugeX 0 hugeW 1 "black"]
      renderDefault scene `shouldBe` Left (InvalidRectangleGeometry hugeX 0 hugeW 1)

    it "rejects an enormous but finite scene instead of hanging" $ do
      let scene = emptyScene 1.0e12 1.0e12 "transparent"
      render scene (AsciiOptions 8 16) `shouldBe` Left (SceneTooLarge 1.0e12 1.0e12)

    it "rejects a zero-width, enormous-height scene instead of hanging (product-only check bypass)" $ do
      let scene = emptyScene 0 1.0e13 "transparent"
      render scene (AsciiOptions 8 16) `shouldBe` Left (SceneTooLarge 0 1.0e13)

    it "rejects an enormous-width, zero-height scene instead of hanging (product-only check bypass)" $ do
      let scene = emptyScene 1.0e13 0 "transparent"
      render scene (AsciiOptions 8 16) `shouldBe` Left (SceneTooLarge 1.0e13 0)

  describe "stroked rect" $ do
    it "draws box-drawing corners and edges" $ do
      let scene = withInstructions (emptyScene 24 32 "transparent")
            [(makeRect 0 0 16 16 "") { prStroke = "#000000", prStrokeWidth = 1 }]
      render scene (AsciiOptions 8 16) `shouldBe` Right "\x250C\x2500\x2510\n\x2514\x2500\x2518"

  describe "glyph_run" $ do
    it "places literal characters at their scene positions" $ do
      let scene = withInstructions (emptyScene 16 16 "transparent")
            [makeGlyphRun
              [ PaintGlyphPlacement (fromEnum 'h') 0 0
              , PaintGlyphPlacement (fromEnum 'i') 8 0
              ]
              "terminal-mono" 16 "#000000"]
      render scene (AsciiOptions 8 16) `shouldBe` Right "hi"

    it "maps unsafe control code points to a placeholder" $ do
      let scene = withInstructions (emptyScene 16 16 "transparent")
            [makeGlyphRun [PaintGlyphPlacement 0x07 0 0] "terminal-mono" 16 "#000000"]
      render scene (AsciiOptions 8 16) `shouldBe` Right "?"

    it "maps a UTF-16 surrogate code point to a placeholder" $ do
      let scene = withInstructions (emptyScene 16 16 "transparent")
            [makeGlyphRun [PaintGlyphPlacement 0xDC80 0 0] "terminal-mono" 16 "#000000"]
      render scene (AsciiOptions 8 16) `shouldBe` Right "?"

    it "skips a glyph with a non-finite position instead of failing the render" $ do
      let scene = withInstructions (emptyScene 16 16 "transparent")
            [makeGlyphRun
              [ PaintGlyphPlacement (fromEnum 'h') (1 / 0) 0
              , PaintGlyphPlacement (fromEnum 'i') 8 0
              ]
              "terminal-mono" 16 "#000000"]
      render scene (AsciiOptions 8 16) `shouldBe` Right " i"

  describe "line" $ do
    it "draws a horizontal box-drawing run" $ do
      let scene = withInstructions (emptyScene 32 16 "transparent")
            [makeLine 0 0 24 0 "#000000" 1]
      render scene (AsciiOptions 8 16) `shouldBe` Right "\x2500\x2500\x2500\x2500"

    it "draws a vertical box-drawing run" $ do
      let scene = withInstructions (emptyScene 8 48 "transparent")
            [makeLine 0 0 0 32 "#000000" 1]
      render scene (AsciiOptions 8 16) `shouldBe` Right "\x2502\n\x2502\n\x2502"

    it "rejects a line with a non-finite coordinate" $ do
      let scene = withInstructions (emptyScene 8 8 "transparent")
            [makeLine (1 / 0) 0 8 8 "#000000" 1]
      render scene (AsciiOptions 8 8) `shouldBe` Left (InvalidLineGeometry (1 / 0) 0 8 8)

    it "clamps an enormous but finite diagonal line to the clip bounds instead of hanging" $ do
      let scene = withInstructions (emptyScene 8 8 "transparent")
            [makeLine 0 0 1.0e12 1.0e12 "#000000" 1]
      case render scene (AsciiOptions 8 8) of
        Right text -> length text `shouldSatisfy` (<= 3)
        Left err -> expectationFailure ("expected a bounded render, got " ++ show err)

  describe "rect fill/stroke bounds" $
    it "clamps an enormous but finite rectangle to the clip bounds instead of hanging" $ do
      let scene = withInstructions (emptyScene 8 8 "transparent")
            [makeRect 0 0 1.0e12 1.0e12 "#000000"]
      render scene (AsciiOptions 8 8) `shouldBe` Right "\x2588"

  describe "group" $ do
    it "recurses into its children" $ do
      let scene = withInstructions (emptyScene 16 16 "transparent")
            [makeGroup [makeRect 0 0 8 16 "#000000"]]
      render scene (AsciiOptions 8 16) `shouldBe` Right "\x2588\x2588"

  describe "clip" $ do
    it "drops children outside the clip rectangle" $ do
      let scene = withInstructions (emptyScene 16 16 "transparent")
            [makeClip 0 0 8 16
              [makeGlyphRun
                [ PaintGlyphPlacement (fromEnum 'a') 0 0
                , PaintGlyphPlacement (fromEnum 'b') 8 0
                ]
                "terminal-mono" 16 "#000000"]]
      render scene (AsciiOptions 8 16) `shouldBe` Right "a"

    it "rejects a clip with a non-finite coordinate" $ do
      let scene = withInstructions (emptyScene 16 16 "transparent")
            [makeClip (1 / 0) 0 8 16 []]
      render scene (AsciiOptions 8 16) `shouldBe` Left (InvalidClipGeometry (1 / 0) 0 8 16)

    it "rejects a clip whose individually-finite x+w overflows to infinity" $ do
      let hugeX = 1.7e308 :: Double
          hugeW = 1.0e308 :: Double
          scene = withInstructions (emptyScene 16 16 "transparent") [makeClip hugeX 0 hugeW 16 []]
      render scene (AsciiOptions 8 16) `shouldBe` Left (InvalidClipGeometry hugeX 0 hugeW 16)

    it "does not let a large-but-finite clip extent unclamp a nested rect's fill range" $ do
      -- pclW here is finite and passes validClip's own checks (individually
      -- and summed with pclX); before toCell saturated its output, this
      -- exact magnitude rounded to `minBound :: Int`, which then flowed
      -- through intersectClip into a nested rect's clampCol/clampRow and
      -- wrapped `clMaxCol - 1` from minBound to maxBound, unclamping the
      -- rect's fill range entirely.
      let hugeButFinite = 6.6461399789245786e35 :: Double
          scene = withInstructions (emptyScene 800 16 "transparent")
            [makeClip 0 0 hugeButFinite 16
              [makeRect 0 0 1.0e19 16 "#000000"]]
      case render scene (AsciiOptions 8 16) of
        Right text -> length text `shouldSatisfy` (<= 100)
        Left err -> expectationFailure ("expected a bounded render, got " ++ show err)

  describe "layer" $ do
    it "recurses into its children when plain" $ do
      let scene = withInstructions (emptyScene 16 16 "transparent")
            [makeLayer [makeRect 0 0 8 16 "#000000"]]
      render scene (AsciiOptions 8 16) `shouldBe` Right "\x2588\x2588"

    it "rejects a layer with filters" $ do
      let scene = withInstructions (emptyScene 16 16 "transparent")
            [(makeLayer []) { plyHasFilters = True }]
      render scene (AsciiOptions 8 16) `shouldBe` Left (UnsupportedInstruction "layer with filters")

withInstructions :: PaintScene -> [PaintInstruction] -> PaintScene
withInstructions scene instructions = scene { psInstructions = instructions }

isInvalidDimensions :: Either PaintVmAsciiError String -> Bool
isInvalidDimensions (Left (InvalidSceneDimensions width height)) = isNaN width && height == 1
isInvalidDimensions _ = False
