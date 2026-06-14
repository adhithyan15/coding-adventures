-- | DataMatrixSpec — hspec tests for CodingAdventures.DataMatrix.
--
-- Tests cover:
--
-- * Version string constant
-- * Basic encoding (returns Right)
-- * Correct symbol dimensions
-- * Grid structure invariants (L-finder)
-- * Determinism
-- * Symbol size auto-selection
-- * InputTooLong error for excess data
-- * Specific known symbol sizes
-- * Rectangular symbol selection
-- * encodeAt (forced size)
module DataMatrixSpec (spec) where

import Test.Hspec
import qualified Data.Vector as V
import CodingAdventures.DataMatrix
import CodingAdventures.Barcode2D
  ( ModuleGrid (..)
  , Barcode2DLayoutConfig (..)
  , mgRows, mgCols, mgModules
  )

spec :: Spec
spec = do

  -- --------------------------------------------------------------------------
  -- Version string
  -- --------------------------------------------------------------------------
  describe "version" $ do
    it "is \"0.1.0\"" $
      version `shouldBe` "0.1.0"

  -- --------------------------------------------------------------------------
  -- Basic encode — returns Right
  -- --------------------------------------------------------------------------
  describe "encode \"A\" defaultOptions" $ do
    let result = encode "A" defaultOptions

    it "returns Right (no error)" $
      case result of
        Right _ -> return ()
        Left e  -> expectationFailure ("Expected Right, got Left: " ++ show e)

    it "produces a 10×10 grid (smallest square for 1 codeword)" $
      case result of
        Right g -> do
          mgRows g `shouldBe` 10
          mgCols g `shouldBe` 10
        Left e  -> expectationFailure (show e)

    it "has exactly rows*cols modules" $
      case result of
        Right g -> V.length (mgModules g) `shouldBe` mgRows g * mgCols g
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- L-finder structural invariants
  -- --------------------------------------------------------------------------
  describe "L-finder border (10×10 symbol for \"A\")" $ do
    let Right grid = encode "A" defaultOptions
    let sz = mgRows grid  -- 10

    it "left column (col 0) is all dark" $ do
      let leftDark = all (\r -> mgModules grid V.! (r * mgCols grid + 0)) [0..sz-1]
      leftDark `shouldBe` True

    it "bottom row (row sz-1) is all dark" $ do
      let botDark = all (\c -> mgModules grid V.! ((sz-1) * mgCols grid + c)) [0..sz-1]
      botDark `shouldBe` True

    it "top row alternates starting dark at col 0" $ do
      let topDark col = mgModules grid V.! (0 * mgCols grid + col)
      topDark 0 `shouldBe` True   -- col 0: dark
      topDark 1 `shouldBe` False  -- col 1: light
      topDark 2 `shouldBe` True   -- col 2: dark

    it "right column alternates starting dark at row 0" $ do
      let rightDark row = mgModules grid V.! (row * mgCols grid + (sz - 1))
      rightDark 0 `shouldBe` True   -- row 0: dark
      rightDark 1 `shouldBe` False  -- row 1: light
      rightDark 2 `shouldBe` True   -- row 2: dark

  -- --------------------------------------------------------------------------
  -- Determinism
  -- --------------------------------------------------------------------------
  describe "determinism" $ do
    it "encode \"HELLO\" produces the same grid twice" $ do
      let r1 = encode "HELLO" defaultOptions
      let r2 = encode "HELLO" defaultOptions
      r1 `shouldBe` r2

    it "encode \"12345\" produces the same grid twice" $ do
      let r1 = encode "12345" defaultOptions
      let r2 = encode "12345" defaultOptions
      r1 `shouldBe` r2

  -- --------------------------------------------------------------------------
  -- Auto-symbol size selection
  -- --------------------------------------------------------------------------
  describe "auto-symbol size selection" $ do
    it "\"A\" (1 codeword) → 10×10" $ do
      case encode "A" defaultOptions of
        Right g -> (mgRows g, mgCols g) `shouldBe` (10, 10)
        Left e  -> expectationFailure (show e)

    it "short digit string \"12\" → 10×10 (digit-pair compaction: 1 codeword)" $ do
      case encode "12" defaultOptions of
        Right g -> (mgRows g, mgCols g) `shouldBe` (10, 10)
        Left e  -> expectationFailure (show e)

    it "\"HELLO WORLD\" uses at least 12×12" $ do
      case encode "HELLO WORLD" defaultOptions of
        Right g -> mgRows g `shouldSatisfy` (>= 12)
        Left e  -> expectationFailure (show e)

    it "a long string produces a larger symbol than a short one" $ do
      case (encode "A" defaultOptions, encode (replicate 50 'A') defaultOptions) of
        (Right g1, Right g2) ->
          mgRows g1 `shouldSatisfy` (< mgRows g2)
        _ -> expectationFailure "encode failed"

  -- --------------------------------------------------------------------------
  -- Digit-pair compression
  -- --------------------------------------------------------------------------
  describe "digit-pair compaction" $ do
    it "\"99\" (1 codeword = 229) fits in 10×10" $ do
      case encode "99" defaultOptions of
        Right g -> (mgRows g, mgCols g) `shouldBe` (10, 10)
        Left e  -> expectationFailure (show e)

    it "\"1234\" (2 codewords) fits in 10×10 (capacity 3)" $ do
      case encode "1234" defaultOptions of
        Right g -> (mgRows g, mgCols g) `shouldBe` (10, 10)
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- InputTooLong error
  -- --------------------------------------------------------------------------
  describe "InputTooLong" $ do
    it "a 2000-character string returns Left InputTooLong" $ do
      let input = replicate 2000 'A'
      case encode input defaultOptions of
        Left (InputTooLong _) -> return ()
        Left other            -> expectationFailure ("Wrong error: " ++ show other)
        Right _               -> expectationFailure "Expected Left InputTooLong"

  -- --------------------------------------------------------------------------
  -- encodeAt — forced symbol size
  -- --------------------------------------------------------------------------
  describe "encodeAt" $ do
    it "encodeAt \"A\" 10 10 produces a 10×10 grid" $ do
      case encodeAt "A" 10 10 of
        Right g -> (mgRows g, mgCols g) `shouldBe` (10, 10)
        Left e  -> expectationFailure (show e)

    it "encodeAt \"A\" 12 12 produces a 12×12 grid" $ do
      case encodeAt "A" 12 12 of
        Right g -> (mgRows g, mgCols g) `shouldBe` (12, 12)
        Left e  -> expectationFailure (show e)

    it "encodeAt returns InvalidSymbolSize for an invalid size" $ do
      case encodeAt "A" 11 11 of
        Left (InvalidSymbolSize _) -> return ()
        Left other                 -> expectationFailure ("Wrong error: " ++ show other)
        Right _                    -> expectationFailure "Expected Left InvalidSymbolSize"

    it "encodeAt returns InputTooLong when data doesn't fit" $ do
      case encodeAt (replicate 100 'A') 10 10 of
        Left (InputTooLong _) -> return ()
        Left other            -> expectationFailure ("Wrong error: " ++ show other)
        Right _               -> expectationFailure "Expected Left InputTooLong"

  -- --------------------------------------------------------------------------
  -- Rectangular symbols
  -- --------------------------------------------------------------------------
  describe "Rectangular symbols" $ do
    let rectOpts = defaultOptions { dmShape = Rectangular }

    it "\"A\" in Rectangular mode → 8×18 symbol" $ do
      case encode "A" rectOpts of
        Right g -> (mgRows g, mgCols g) `shouldBe` (8, 18)
        Left e  -> expectationFailure (show e)

    it "grid dimensions match for rectangular symbol" $ do
      case encode "A" rectOpts of
        Right g -> V.length (mgModules g) `shouldBe` mgRows g * mgCols g
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- AnyShape mode
  -- --------------------------------------------------------------------------
  describe "AnyShape mode" $ do
    let anyOpts = defaultOptions { dmShape = AnyShape }

    it "encode \"A\" AnyShape returns Right" $ do
      case encode "A" anyOpts of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)

    it "AnyShape result dimensions are at least 8×8" $ do
      case encode "A" anyOpts of
        Right g -> do
          mgRows g `shouldSatisfy` (>= 8)
          mgCols g `shouldSatisfy` (>= 8)
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- Module count sanity
  -- --------------------------------------------------------------------------
  describe "module count" $ do
    it "all modules are Bool (trivially always True in Haskell)" $ do
      case encode "TEST" defaultOptions of
        Right g ->
          -- Just verify we can traverse all modules without error.
          V.length (mgModules g) `shouldBe` mgRows g * mgCols g
        Left e  -> expectationFailure (show e)

    it "grid has at least one dark module for any input" $ do
      case encode "A" defaultOptions of
        Right g ->
          V.any id (mgModules g) `shouldBe` True
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- Extended symbol sizes
  -- --------------------------------------------------------------------------
  describe "larger symbols" $ do
    it "3 codewords fit in 10×10 (capacity=3)" $ do
      -- "ABC" → [66,67,68] — exactly 3 codewords
      case encode "ABC" defaultOptions of
        Right g -> (mgRows g, mgCols g) `shouldBe` (10, 10)
        Left e  -> expectationFailure (show e)

    it "4 codewords require at least 12×12" $ do
      -- "ABCD" → [66,67,68,69] — 4 codewords, needs 12×12 (capacity 5)
      case encode "ABCD" defaultOptions of
        Right g -> mgRows g `shouldSatisfy` (>= 12)
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- encodeAndLayout round-trip
  -- --------------------------------------------------------------------------
  describe "encodeAndLayout" $ do
    it "returns Right for a simple input" $ do
      let cfg = Barcode2DLayoutConfig
            { moduleSizePx     = 10
            , quietZoneModules = 1
            , foreground       = "#000000"
            , background       = "#ffffff"
            }
      case encodeAndLayout "A" defaultOptions cfg of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)
