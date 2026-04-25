-- | Unit tests for CodingAdventures.Barcode2D.
--
-- Tests verify:
--   1. emptyGrid — dimensions, all-false modules, shape
--   2. setModule — immutability, correct index update, bounds checking
--   3. defaultConfig — default field values
--   4. layoutSquare — scene dimensions, instruction count, background
--   5. layoutHex — scene dimensions, hex path structure
--   6. layout dispatch — routes to square or hex based on mgShape
--   7. layout validation — rejects invalid config values
module Barcode2DSpec (spec) where

import Test.Hspec
import Control.Exception (evaluate, try, SomeException)
import qualified Data.Vector as V
import CodingAdventures.Barcode2D
import CodingAdventures.PaintInstructions
  ( PaintScene (..)
  , PaintInstruction (..)
  , PathCommand (..)
  )

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

-- | Get the value at (row, col) in a ModuleGrid.
getModule :: ModuleGrid -> Int -> Int -> Bool
getModule grid row col =
  mgModules grid V.! (row * mgCols grid + col)

-- ---------------------------------------------------------------------------
-- Spec
-- ---------------------------------------------------------------------------

spec :: Spec
spec = do

  -- -------------------------------------------------------------------------
  -- emptyGrid
  -- -------------------------------------------------------------------------
  describe "emptyGrid" $ do

    it "stores correct row and column counts" $ do
      let g = emptyGrid 7 5 Square
      mgRows g `shouldBe` 7
      mgCols g `shouldBe` 5

    it "all modules are False (light)" $ do
      let g = emptyGrid 4 4 Square
      V.all (== False) (mgModules g) `shouldBe` True

    it "vector length = rows * cols" $ do
      let g = emptyGrid 6 3 Square
      V.length (mgModules g) `shouldBe` 18

    it "stores Square shape" $ do
      mgShape (emptyGrid 5 5 Square) `shouldBe` Square

    it "stores Hex shape" $ do
      mgShape (emptyGrid 33 30 Hex) `shouldBe` Hex

    it "1x1 grid has one False module" $ do
      let g = emptyGrid 1 1 Square
      V.length (mgModules g) `shouldBe` 1
      mgModules g V.! 0 `shouldBe` False

  -- -------------------------------------------------------------------------
  -- setModule
  -- -------------------------------------------------------------------------
  describe "setModule" $ do

    it "sets a module to True" $ do
      let g  = emptyGrid 3 3 Square
      let g2 = setModule g 1 1 True
      getModule g2 1 1 `shouldBe` True

    it "sets a module to False (light)" $ do
      let g  = emptyGrid 3 3 Square
      let g1 = setModule g 0 0 True
      let g2 = setModule g1 0 0 False
      getModule g2 0 0 `shouldBe` False

    it "does not mutate the original grid" $ do
      let g  = emptyGrid 3 3 Square
      let g2 = setModule g 1 1 True
      -- g is unchanged
      getModule g  1 1 `shouldBe` False
      getModule g2 1 1 `shouldBe` True

    it "only changes the target module (other modules unchanged)" $ do
      let g  = emptyGrid 3 3 Square
      let g2 = setModule g 2 2 True
      -- All modules except (2,2) are still False
      let others = [ (r, c) | r <- [0..2], c <- [0..2], not (r == 2 && c == 2) ]
      all (\(r, c) -> not (getModule g2 r c)) others `shouldBe` True

    it "can set top-left corner (0,0)" $ do
      let g  = emptyGrid 5 5 Square
      let g2 = setModule g 0 0 True
      getModule g2 0 0 `shouldBe` True

    it "can set bottom-right corner (rows-1, cols-1)" $ do
      let g  = emptyGrid 5 5 Square
      let g2 = setModule g 4 4 True
      getModule g2 4 4 `shouldBe` True

    it "throws on row out of bounds (negative)" $ do
      let g = emptyGrid 3 3 Square
      result <- try (evaluate (setModule g (-1) 0 True)) :: IO (Either SomeException ModuleGrid)
      case result of
        Left  _ -> return ()   -- expected
        Right _ -> fail "should have thrown"

    it "throws on row out of bounds (too large)" $ do
      let g = emptyGrid 3 3 Square
      result <- try (evaluate (setModule g 3 0 True)) :: IO (Either SomeException ModuleGrid)
      case result of
        Left  _ -> return ()
        Right _ -> fail "should have thrown"

    it "throws on col out of bounds (negative)" $ do
      let g = emptyGrid 3 3 Square
      result <- try (evaluate (setModule g 0 (-1) True)) :: IO (Either SomeException ModuleGrid)
      case result of
        Left  _ -> return ()
        Right _ -> fail "should have thrown"

    it "throws on col out of bounds (too large)" $ do
      let g = emptyGrid 3 3 Square
      result <- try (evaluate (setModule g 0 3 True)) :: IO (Either SomeException ModuleGrid)
      case result of
        Left  _ -> return ()
        Right _ -> fail "should have thrown"

    it "multiple setModule calls compose correctly" $ do
      let g  = emptyGrid 3 3 Square
      let g2 = setModule (setModule (setModule g 0 0 True) 1 1 True) 2 2 True
      getModule g2 0 0 `shouldBe` True
      getModule g2 1 1 `shouldBe` True
      getModule g2 2 2 `shouldBe` True
      -- Off-diagonal modules still False
      getModule g2 0 1 `shouldBe` False
      getModule g2 1 2 `shouldBe` False

  -- -------------------------------------------------------------------------
  -- defaultConfig
  -- -------------------------------------------------------------------------
  describe "defaultConfig" $ do

    it "moduleSizePx = 10" $
      moduleSizePx defaultConfig `shouldBe` 10

    it "quietZoneModules = 4" $
      quietZoneModules defaultConfig `shouldBe` 4

    it "foreground = #000000" $
      foreground defaultConfig `shouldBe` "#000000"

    it "background = #ffffff" $
      background defaultConfig `shouldBe` "#ffffff"

  -- -------------------------------------------------------------------------
  -- layoutSquare
  -- -------------------------------------------------------------------------
  describe "layoutSquare" $ do

    it "all-dark 1x1 grid with no quiet zone has width = moduleSizePx" $ do
      let g   = setModule (emptyGrid 1 1 Square) 0 0 True
      let cfg = defaultConfig { quietZoneModules = 0, moduleSizePx = 8 }
      let s   = layout g cfg
      psWidth  s `shouldBe` 8.0
      psHeight s `shouldBe` 8.0

    it "empty 3x3 grid at default config has correct dimensions" $ do
      -- totalWidth = (3 + 2*4) * 10 = 110
      let g = emptyGrid 3 3 Square
      let s = layout g defaultConfig
      psWidth  s `shouldBe` 110.0
      psHeight s `shouldBe` 110.0

    it "5x5 all-dark grid has 26 instructions (1 bg + 25 dark rects)" $ do
      let g   = foldr (\(r,c) acc -> setModule acc r c True)
                       (emptyGrid 5 5 Square)
                       [(r, c) | r <- [0..4], c <- [0..4]]
      let cfg = defaultConfig { quietZoneModules = 0 }
      let s   = layout g cfg
      -- 1 background rect + 25 module rects
      length (psInstructions s) `shouldBe` 26

    it "empty grid has exactly 1 instruction (background rect)" $ do
      let g   = emptyGrid 5 5 Square
      let cfg = defaultConfig { quietZoneModules = 0 }
      let s   = layout g cfg
      length (psInstructions s) `shouldBe` 1

    it "background rect is the first instruction" $ do
      let g = emptyGrid 3 3 Square
      let s = layout g defaultConfig
      case head (psInstructions s) of
        PaintRect { prX = x, prY = y } -> do
          x `shouldBe` 0.0
          y `shouldBe` 0.0
        _ -> fail "expected PaintRect as first instruction"

    it "background rect has correct fill color" $ do
      let g   = emptyGrid 3 3 Square
      let cfg = defaultConfig { background = "#eeeeee" }
      let s   = layout g cfg
      case head (psInstructions s) of
        PaintRect { prFill = f } -> f `shouldBe` "#eeeeee"
        _                        -> fail "expected PaintRect"

    it "scene background matches config background" $ do
      let g   = emptyGrid 3 3 Square
      let cfg = defaultConfig { background = "#ffeecc" }
      let s   = layout g cfg
      psBg s `shouldBe` "#ffeecc"

    it "dark module rect has foreground color" $ do
      let g   = setModule (emptyGrid 1 1 Square) 0 0 True
      let cfg = defaultConfig { quietZoneModules = 0, foreground = "#cc0000" }
      let s   = layout g cfg
      -- instructions: [bg, darkRect]
      let instrs = psInstructions s
      length instrs `shouldBe` 2
      case instrs !! 1 of
        PaintRect { prFill = f } -> f `shouldBe` "#cc0000"
        _                        -> fail "expected PaintRect for dark module"

    it "module rect size equals moduleSizePx" $ do
      let g   = setModule (emptyGrid 1 1 Square) 0 0 True
      let cfg = defaultConfig { quietZoneModules = 0, moduleSizePx = 15 }
      let s   = layout g cfg
      case psInstructions s !! 1 of
        PaintRect { prW = w, prH = h } -> do
          w `shouldBe` 15.0
          h `shouldBe` 15.0
        _ -> fail "expected PaintRect"

    it "module at (0,0) with quiet zone is offset correctly" $ do
      -- quietZonePx = 4 * 10 = 40; module at (0,0) should be at x=40, y=40
      let g   = setModule (emptyGrid 1 1 Square) 0 0 True
      let s   = layout g defaultConfig
      case psInstructions s !! 1 of
        PaintRect { prX = x, prY = y } -> do
          x `shouldBe` 40.0
          y `shouldBe` 40.0
        _ -> fail "expected PaintRect"

    it "module at (row=1, col=2) is placed at correct pixel position" $ do
      -- With moduleSizePx=10, quietZone=0:
      --   x = 0 + 2*10 = 20
      --   y = 0 + 1*10 = 10
      let g   = setModule (emptyGrid 3 4 Square) 1 2 True
      let cfg = defaultConfig { quietZoneModules = 0 }
      let s   = layout g cfg
      case psInstructions s !! 1 of
        PaintRect { prX = x, prY = y } -> do
          x `shouldBe` 20.0
          y `shouldBe` 10.0
        _ -> fail "expected PaintRect"

    it "layout and layoutSquare give same result for Square grid" $ do
      let g = setModule (emptyGrid 3 3 Square) 0 0 True
      layout g defaultConfig `shouldBe` layoutSquare g defaultConfig

  -- -------------------------------------------------------------------------
  -- layoutHex
  -- -------------------------------------------------------------------------
  describe "layoutHex" $ do

    it "produces a PaintPath for each dark module" $ do
      let g   = setModule (emptyGrid 2 2 Hex) 0 0 True
      let cfg = defaultConfig { quietZoneModules = 0 }
      let s   = layout g cfg
      -- 1 bg rect + 1 hex path
      length (psInstructions s) `shouldBe` 2
      case psInstructions s !! 1 of
        PaintPath {} -> return ()
        _            -> fail "expected PaintPath for hex module"

    it "each hex path has exactly 7 commands (MoveTo + 5 LineTo + ClosePath)" $ do
      -- buildHexPath: MoveTo + 5 LineTo + ClosePath = 7 commands total
      let g   = setModule (emptyGrid 1 1 Hex) 0 0 True
      let cfg = defaultConfig { quietZoneModules = 0 }
      let s   = layout g cfg
      case psInstructions s !! 1 of
        PaintPath { ppCommands = cmds } -> length cmds `shouldBe` 7
        _                               -> fail "expected PaintPath"

    it "hex path starts with MoveTo and ends with ClosePath" $ do
      let g   = setModule (emptyGrid 1 1 Hex) 0 0 True
      let cfg = defaultConfig { quietZoneModules = 0 }
      let s   = layout g cfg
      case psInstructions s !! 1 of
        PaintPath { ppCommands = cmds } -> do
          case head cmds of
            MoveTo _ _ -> pure () :: IO ()
            _          -> fail "first command should be MoveTo"
          case last cmds of
            ClosePath -> pure () :: IO ()
            _         -> fail "last command should be ClosePath"
        _ -> fail "expected PaintPath"

    it "hex path middle commands are all LineTo" $ do
      let g   = setModule (emptyGrid 1 1 Hex) 0 0 True
      let cfg = defaultConfig { quietZoneModules = 0 }
      let s   = layout g cfg
      case psInstructions s !! 1 of
        PaintPath { ppCommands = cmds } -> do
          -- Drop MoveTo (first) and ClosePath (last); all remaining are LineTo
          let middle = init (tail cmds)
          length middle `shouldBe` 5
          all (\c -> case c of { LineTo _ _ -> True; _ -> False }) middle
            `shouldBe` True
        _ -> fail "expected PaintPath"

    it "empty hex grid produces only background rect" $ do
      let g   = emptyGrid 3 3 Hex
      let cfg = defaultConfig { quietZoneModules = 0 }
      let s   = layout g cfg
      length (psInstructions s) `shouldBe` 1

    it "layout and layoutHex give same result for Hex grid" $ do
      let g   = setModule (emptyGrid 3 3 Hex) 1 1 True
      layout g defaultConfig `shouldBe` layoutHex g defaultConfig

    it "odd row is shifted right by hexWidth/2" $ do
      -- Row 1 is odd → cx shifted right by moduleSizePx/2 vs row 0 same col
      let g0  = setModule (emptyGrid 2 1 Hex) 0 0 True   -- even row
      let g1  = setModule (emptyGrid 2 1 Hex) 1 0 True   -- odd row
      let cfg = defaultConfig { quietZoneModules = 0, moduleSizePx = 10 }

      let getFirstMoveToX scn =
            case psInstructions scn !! 1 of
              PaintPath { ppCommands = (MoveTo x _ : _) } -> x
              _ -> error "expected PaintPath with MoveTo"

      let s0 = layout g0 cfg
      let s1 = layout g1 cfg

      -- Odd row center is shifted right by 5 (= 10/2)
      -- The difference in MoveTo x should equal 5.0
      let circumR = 10.0 / sqrt 3
      getFirstMoveToX s1 - getFirstMoveToX s0 `shouldBe` 5.0 + circumR - circumR

  -- -------------------------------------------------------------------------
  -- layout validation
  -- -------------------------------------------------------------------------
  describe "layout validation" $ do

    it "throws when moduleSizePx = 0" $ do
      let g   = emptyGrid 1 1 Square
      let cfg = defaultConfig { moduleSizePx = 0 }
      result <- try (evaluate (psWidth (layout g cfg))) :: IO (Either SomeException Double)
      case result of
        Left  _ -> return ()
        Right _ -> fail "should have thrown for moduleSizePx = 0"

    it "throws when moduleSizePx < 0" $ do
      let g   = emptyGrid 1 1 Square
      let cfg = defaultConfig { moduleSizePx = -5 }
      result <- try (evaluate (psWidth (layout g cfg))) :: IO (Either SomeException Double)
      case result of
        Left  _ -> return ()
        Right _ -> fail "should have thrown for negative moduleSizePx"

    it "throws when quietZoneModules < 0" $ do
      let g   = emptyGrid 1 1 Square
      let cfg = defaultConfig { quietZoneModules = -1 }
      result <- try (evaluate (psWidth (layout g cfg))) :: IO (Either SomeException Double)
      case result of
        Left  _ -> return ()
        Right _ -> fail "should have thrown for negative quietZoneModules"

    it "allows quietZoneModules = 0" $ do
      let g   = emptyGrid 1 1 Square
      let cfg = defaultConfig { quietZoneModules = 0 }
      -- Should not throw
      let s = layout g cfg
      psWidth s `shouldBe` 10.0

  -- -------------------------------------------------------------------------
  -- QR-Code-like 21x21 grid sanity check
  -- -------------------------------------------------------------------------
  describe "21x21 QR-like grid" $ do

    it "default layout produces 290x290 px scene" $ do
      -- totalWidth = (21 + 2*4) * 10 = 290
      let g = emptyGrid 21 21 Square
      let s = layout g defaultConfig
      psWidth  s `shouldBe` 290.0
      psHeight s `shouldBe` 290.0

    it "all-dark 21x21 grid has 442 instructions (1 bg + 441 dark)" $ do
      let g = foldr (\(r,c) acc -> setModule acc r c True)
                    (emptyGrid 21 21 Square)
                    [(r, c) | r <- [0..20], c <- [0..20]]
      let cfg = defaultConfig { quietZoneModules = 0 }
      let s   = layout g cfg
      length (psInstructions s) `shouldBe` 442
