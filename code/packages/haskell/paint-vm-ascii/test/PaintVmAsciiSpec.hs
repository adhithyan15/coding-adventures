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
import Data.List (sortOn)
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

  -- Bresenham regression suite (issue #12093). The diagonal-line recursion
  -- used to seed its error term with 0 instead of deltaCol - deltaRow, which
  -- made slopes such as (dx=1, dy=3) or (dx=3, dy=1) overshoot the endpoint
  -- and recurse forever. See 'assertBresenhamPath' below for how the path
  -- property is read back out of the public 'render' output.
  describe "line (Bresenham, issue #12093)" $ do
    it "terminates on a shallow line (dRow=1, dCol=3) with the exact Bresenham cells" $ do
      renderLineCells (0, 0) (1, 3) `shouldBe` Right "\x2500\x2500\n  \x2500\x2500"
      assertBresenhamPath (0, 0) (1, 3)

    it "terminates on the same shallow line drawn in reverse" $ do
      renderLineCells (1, 3) (0, 0) `shouldBe` Right "\x2500\x2500\n  \x2500\x2500"
      assertBresenhamPath (1, 3) (0, 0)

    it "terminates on a steep line (dx=1, dy=3) with the exact Bresenham cells" $ do
      renderLineCells (0, 0) (3, 1) `shouldBe` Right "\x2502\n\x2502\n \x2502\n \x2502"
      assertBresenhamPath (0, 0) (3, 1)

    it "terminates on the same steep line drawn in reverse" $ do
      renderLineCells (3, 1) (0, 0) `shouldBe` Right "\x2502\n\x2502\n \x2502\n \x2502"
      assertBresenhamPath (3, 1) (0, 0)

    it "covers all eight octants from a central point" $
      mapM_ (\(dRow, dCol) -> assertBresenhamPath (7, 7) (7 + dRow, 7 + dCol))
        [(2, 5), (5, 2), (5, -2), (2, -5), (-2, -5), (-5, -2), (-5, 2), (-2, 5)]

    it "covers the four 45-degree diagonals" $
      mapM_ (assertBresenhamPath (7, 7)) [(11, 11), (11, 3), (3, 3), (3, 11)]

    it "keeps the path property for horizontal, vertical and single-point lines" $ do
      assertBresenhamPath (4, 2) (4, 12)
      assertBresenhamPath (4, 12) (4, 2)
      assertBresenhamPath (2, 4) (12, 4)
      assertBresenhamPath (12, 4) (2, 4)
      assertBresenhamPath (5, 5) (5, 5)
      renderLineCells (0, 5) (0, 5) `shouldBe` Right "     \x2500"

    it "yields a valid path for every endpoint within 6 cells of the centre" $
      sequence_
        [ assertBresenhamPath (7, 7) (7 + dRow, 7 + dCol)
        | dRow <- [-6 .. 6]
        , dCol <- [-6 .. 6]
        ]

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

-- | A 16x16-cell scene holding one line from @(row0, col0)@ to
-- @(row1, col1)@, rendered at one scene unit per character cell.
renderLineCells :: (Int, Int) -> (Int, Int) -> Either PaintVmAsciiError String
renderLineCells (row0, col0) (row1, col1) =
  render
    (withInstructions (emptyScene 16 16 "transparent")
      [makeLine (fromIntegral col0) (fromIntegral row0) (fromIntegral col1) (fromIntegral row1) "#000000" 1])
    (AsciiOptions 1 1)

-- | Every non-space character in the rendered text, as @(row, col)@.
-- 'render' only trims trailing blanks, so each character's position in
-- the text is its true cell coordinate.
drawnCells :: String -> [(Int, Int)]
drawnCells text =
  [ (row, col)
  | (row, line) <- zip [0 ..] (lines text)
  , (col, ch) <- zip [0 ..] line
  , ch /= ' '
  ]

-- | Render p0 -> p1 and check the full Bresenham path property:
--
--   * it starts at p0 and ends at p1,
--   * it has exactly @max |dx| |dy| + 1@ cells,
--   * consecutive cells differ by exactly 1 on the major axis and by at
--     most 1 on the minor axis (8-connected, no gaps, no doubled cells).
--
-- A Bresenham walk advances its major axis by exactly one cell per step,
-- so sorting the drawn cells along the major axis in the direction of
-- travel recovers the walk's order without reaching into the private
-- recursion. No timeout is used: the fixed recursion provably stops after
-- @max |dx| |dy| + 1@ cells; a regression of the seed would hang the suite
-- rather than fail it, which is still unmistakable in CI.
assertBresenhamPath :: (Int, Int) -> (Int, Int) -> Expectation
assertBresenhamPath p0@(row0, col0) p1@(row1, col1) =
  case renderLineCells p0 p1 of
    Left err -> expectationFailure (label ++ ": expected Right, got " ++ show err)
    Right text -> do
      let path = sortOn (\cell -> majorSign * major cell) (drawnCells text)
          steps = zip path (drop 1 path)
      (label, length path) `shouldBe` (label, max (abs dRow) (abs dCol) + 1)
      (label, take 1 path) `shouldBe` (label, [p0])
      (label, take 1 (reverse path)) `shouldBe` (label, [p1])
      [ (label, a, b) | (a, b) <- steps, abs (major b - major a) /= abs majorSign ] `shouldBe` []
      [ (label, a, b) | (a, b) <- steps, abs (minor b - minor a) > 1 ] `shouldBe` []
  where
    label = show p0 ++ " -> " ++ show p1
    dRow = row1 - row0
    dCol = col1 - col0
    colMajor = abs dCol >= abs dRow
    majorSign = signum (if colMajor then dCol else dRow)
    major (row, col) = if colMajor then col else row
    minor (row, col) = if colMajor then row else col

isInvalidDimensions :: Either PaintVmAsciiError String -> Bool
isInvalidDimensions (Left (InvalidSceneDimensions width height)) = isNaN width && height == 1
isInvalidDimensions _ = False
