-- | Unit tests for CodingAdventures.PaintInstructions.
--
-- Tests verify:
--   1. PathCommand construction and equality
--   2. PaintInstruction (PaintRect and PaintPath) fields
--   3. PaintScene structure and defaults
--   4. Builder helpers: emptyScene, makeRect, makePath, addInstruction
module PaintInstructionsSpec (spec) where

import Test.Hspec
import qualified Data.Map.Strict as Map
import CodingAdventures.PaintInstructions

spec :: Spec
spec = do

  -- -------------------------------------------------------------------------
  -- PathCommand
  -- -------------------------------------------------------------------------
  describe "PathCommand" $ do

    it "MoveTo stores x and y" $ do
      let cmd = MoveTo 10.0 20.0
      cmd `shouldBe` MoveTo 10.0 20.0

    it "LineTo stores x and y" $ do
      let cmd = LineTo 100.0 200.0
      cmd `shouldBe` LineTo 100.0 200.0

    it "ClosePath equals itself" $ do
      ClosePath `shouldBe` ClosePath

    it "MoveTo and LineTo at same coords are not equal" $ do
      MoveTo 1.0 2.0 `shouldNotBe` LineTo 1.0 2.0

    it "can build a triangle path" $ do
      let cmds =
            [ MoveTo 50.0 10.0
            , LineTo 90.0 80.0
            , LineTo 10.0 80.0
            , ClosePath
            ]
      length cmds `shouldBe` 4
      head cmds `shouldBe` MoveTo 50.0 10.0
      last cmds `shouldBe` ClosePath

  -- -------------------------------------------------------------------------
  -- PaintInstruction — PaintRect
  -- -------------------------------------------------------------------------
  describe "PaintRect" $ do

    it "stores x, y, width, height, fill" $ do
      let r = PaintRect { prX = 10, prY = 20, prW = 80, prH = 40
                        , prFill = "#2563eb", prMeta = Map.empty }
      prX r    `shouldBe` 10.0
      prY r    `shouldBe` 20.0
      prW r    `shouldBe` 80.0
      prH r    `shouldBe` 40.0
      prFill r `shouldBe` "#2563eb"

    it "allows empty fill (transparent)" $ do
      let r = PaintRect { prX = 0, prY = 0, prW = 100, prH = 50
                        , prFill = "", prMeta = Map.empty }
      prFill r `shouldBe` ""

    it "metadata defaults to empty Map" $ do
      let r = makeRect 0 0 10 10 "#fff"
      prMeta r `shouldBe` Map.empty

    it "two rects with same fields are equal" $ do
      let a = makeRect 5 5 20 20 "#ff0000"
      let b = makeRect 5 5 20 20 "#ff0000"
      a `shouldBe` b

    it "two rects with different fills are not equal" $ do
      let a = makeRect 0 0 10 10 "#000"
      let b = makeRect 0 0 10 10 "#fff"
      a `shouldNotBe` b

  -- -------------------------------------------------------------------------
  -- PaintInstruction — PaintPath
  -- -------------------------------------------------------------------------
  describe "PaintPath" $ do

    it "stores commands and fill" $ do
      let cmds = [MoveTo 0 0, LineTo 10 0, LineTo 5 8, ClosePath]
      let p = PaintPath { ppCommands = cmds, ppFill = "#ef4444"
                        , ppMeta = Map.empty }
      ppCommands p `shouldBe` cmds
      ppFill p     `shouldBe` "#ef4444"

    it "allows empty path (zero commands)" $ do
      let p = PaintPath { ppCommands = [], ppFill = "#000"
                        , ppMeta = Map.empty }
      ppCommands p `shouldBe` []

    it "makePath builder sets commands and fill" $ do
      let cmds = [MoveTo 0 0, ClosePath]
      let p = makePath cmds "#abc"
      ppCommands p `shouldBe` cmds
      ppFill p     `shouldBe` "#abc"
      ppMeta p     `shouldBe` Map.empty

    it "two paths with same content are equal" $ do
      let p1 = makePath [MoveTo 1 2, LineTo 3 4, ClosePath] "#111"
      let p2 = makePath [MoveTo 1 2, LineTo 3 4, ClosePath] "#111"
      p1 `shouldBe` p2

  -- -------------------------------------------------------------------------
  -- PaintScene
  -- -------------------------------------------------------------------------
  describe "PaintScene" $ do

    it "stores width, height, background" $ do
      let s = emptyScene 800 600 "#ffffff"
      psWidth  s `shouldBe` 800.0
      psHeight s `shouldBe` 600.0
      psBg     s `shouldBe` "#ffffff"

    it "emptyScene has no instructions" $ do
      let s = emptyScene 100 100 "#000"
      psInstructions s `shouldBe` []

    it "emptyScene has empty metadata" $ do
      let s = emptyScene 100 100 "#000"
      psMeta s `shouldBe` Map.empty

    it "can construct scene with instructions directly" $ do
      let r = makeRect 0 0 50 50 "#red"
      let s = PaintScene
                { psWidth = 100, psHeight = 100
                , psBg = "#fff"
                , psInstructions = [r]
                , psMeta = Map.empty
                }
      length (psInstructions s) `shouldBe` 1

    it "stores transparent background" $ do
      let s = emptyScene 400 300 "transparent"
      psBg s `shouldBe` "transparent"

  -- -------------------------------------------------------------------------
  -- addInstruction
  -- -------------------------------------------------------------------------
  describe "addInstruction" $ do

    it "appends one instruction" $ do
      let s0 = emptyScene 200 100 "#fff"
      let s1 = addInstruction s0 (makeRect 0 0 10 10 "#000")
      length (psInstructions s1) `shouldBe` 1

    it "appending preserves existing instructions" $ do
      let r1 = makeRect 0  0  10 10 "#f00"
      let r2 = makeRect 10 0  10 10 "#0f0"
      let r3 = makeRect 20 0  10 10 "#00f"
      let s = foldl addInstruction (emptyScene 100 50 "#fff") [r1, r2, r3]
      length (psInstructions s) `shouldBe` 3
      head (psInstructions s) `shouldBe` r1
      last (psInstructions s) `shouldBe` r3

    it "does not mutate the original scene" $ do
      let s0 = emptyScene 100 100 "#fff"
      let _s1 = addInstruction s0 (makeRect 0 0 10 10 "#000")
      psInstructions s0 `shouldBe` []

    it "can add a PaintPath instruction" $ do
      let cmds = [MoveTo 0 0, LineTo 30 0, LineTo 15 26, ClosePath]
      let s0 = emptyScene 100 100 "#fff"
      let s1 = addInstruction s0 (makePath cmds "#green")
      length (psInstructions s1) `shouldBe` 1
      case head (psInstructions s1) of
        PaintPath { ppFill = f } -> f `shouldBe` "#green"
        _                        -> fail "expected PaintPath"

  -- -------------------------------------------------------------------------
  -- makeRect builder
  -- -------------------------------------------------------------------------
  describe "makeRect" $ do

    it "returns a PaintRect with the given fields" $ do
      let r = makeRect 5.5 6.6 77.0 88.0 "#cccccc"
      case r of
        PaintRect { prX = x, prY = y, prW = w, prH = h, prFill = f } -> do
          x `shouldBe` 5.5
          y `shouldBe` 6.6
          w `shouldBe` 77.0
          h `shouldBe` 88.0
          f `shouldBe` "#cccccc"
        _ -> fail "expected PaintRect"

    it "metadata is empty by default" $ do
      case makeRect 0 0 1 1 "#000" of
        PaintRect { prMeta = m } -> m `shouldBe` Map.empty
        _                        -> fail "expected PaintRect"

  -- -------------------------------------------------------------------------
  -- Mixed instruction types in one scene
  -- -------------------------------------------------------------------------
  describe "mixed PaintRect and PaintPath in scene" $ do

    it "can hold both rect and path instructions" $ do
      let rect = makeRect 0 0 100 100 "#ffffff"
      let tri  = makePath [MoveTo 50 10, LineTo 90 80, LineTo 10 80, ClosePath] "#000000"
      let scene = (emptyScene 100 100 "#fff")
                    { psInstructions = [rect, tri] }
      length (psInstructions scene) `shouldBe` 2

    it "first instruction is PaintRect" $ do
      let rect = makeRect 0 0 100 100 "#ffffff"
      let tri  = makePath [MoveTo 50 10, LineTo 90 80, ClosePath] "#000"
      let scene = (emptyScene 100 100 "#fff")
                    { psInstructions = [rect, tri] }
      case head (psInstructions scene) of
        PaintRect {} -> pure () :: IO ()
        _            -> fail "expected PaintRect"

    it "second instruction is PaintPath" $ do
      let rect = makeRect 0 0 100 100 "#ffffff"
      let tri  = makePath [MoveTo 50 10, LineTo 90 80, ClosePath] "#000"
      let scene = (emptyScene 100 100 "#fff")
                    { psInstructions = [rect, tri] }
      case last (psInstructions scene) of
        PaintPath {} -> pure () :: IO ()
        _            -> fail "expected PaintPath"
