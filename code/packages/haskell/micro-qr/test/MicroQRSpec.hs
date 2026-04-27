-- | MicroQRSpec — hspec tests for CodingAdventures.MicroQR.
--
-- Tests cover:
--
-- * Version string constant
-- * Basic encoding (returns Right)
-- * M1-M4 symbol dimensions
-- * Finder pattern structural invariants
-- * Timing pattern invariants
-- * Determinism
-- * Auto-selection by input type
-- * InputTooLong error
-- * InvalidConfiguration error
-- * encodeAt forced version+ECC
-- * ECC level options
-- * Mask selection is consistent
module MicroQRSpec (spec) where

import Test.Hspec
import qualified Data.Vector as V
import CodingAdventures.MicroQR
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
  describe "encode \"1\" Nothing Nothing" $ do
    let result = encode "1" Nothing Nothing

    it "returns Right (no error)" $
      case result of
        Right _ -> return ()
        Left e  -> expectationFailure ("Expected Right, got Left: " ++ show e)

    it "produces an 11×11 grid (M1 = 11×11)" $
      case result of
        Right g -> do
          mgRows g `shouldBe` 11
          mgCols g `shouldBe` 11
        Left e  -> expectationFailure (show e)

    it "has exactly rows*cols modules" $
      case result of
        Right g -> V.length (mgModules g) `shouldBe` mgRows g * mgCols g
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- M1-M4 symbol sizes
  -- --------------------------------------------------------------------------
  describe "symbol sizes" $ do
    it "M1 is 11×11" $
      case encode "1" (Just M1) Nothing of
        Right g -> (mgRows g, mgCols g) `shouldBe` (11, 11)
        Left e  -> expectationFailure (show e)

    it "M2 is 13×13" $
      case encode "1" (Just M2) Nothing of
        Right g -> (mgRows g, mgCols g) `shouldBe` (13, 13)
        Left e  -> expectationFailure (show e)

    it "M3 is 15×15" $
      case encode "1" (Just M3) Nothing of
        Right g -> (mgRows g, mgCols g) `shouldBe` (15, 15)
        Left e  -> expectationFailure (show e)

    it "M4 is 17×17" $
      case encode "1" (Just M4) Nothing of
        Right g -> (mgRows g, mgCols g) `shouldBe` (17, 17)
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- Finder pattern invariants (top-left 7×7 corner)
  -- --------------------------------------------------------------------------
  describe "finder pattern (11×11 for \"1\")" $ do
    let Right grid = encode "1" (Just M1) Nothing
    let gm = mgModules grid
    let sz = mgCols grid  -- 11
    let get r c = gm V.! (r * sz + c)

    it "finder top-left corner (0,0) is dark" $
      get 0 0 `shouldBe` True

    it "finder top-right corner (0,6) is dark" $
      get 0 6 `shouldBe` True

    it "finder bottom-left corner (6,0) is dark" $
      get 6 0 `shouldBe` True

    it "finder bottom-right corner (6,6) is dark" $
      get 6 6 `shouldBe` True

    it "finder top border (row 0) is all dark across cols 0-6" $
      all (\c -> get 0 c) [0..6] `shouldBe` True

    it "finder left border (col 0) is all dark across rows 0-6" $
      all (\r -> get r 0) [0..6] `shouldBe` True

    it "finder inner ring (1,1) to (1,5) is all light" $
      all (\c -> not (get 1 c)) [1..5] `shouldBe` True

    it "finder 3×3 core (rows 2-4, cols 2-4) is all dark" $
      all (\(r, c) -> get r c) [(r, c) | r <- [2..4], c <- [2..4]] `shouldBe` True

  -- --------------------------------------------------------------------------
  -- Timing pattern invariants (Micro QR: row 0 and col 0)
  -- --------------------------------------------------------------------------
  describe "timing patterns" $ do
    let Right grid = encode "1" (Just M1) Nothing
    let gm = mgModules grid
    let sz = mgCols grid  -- 11
    let get r c = gm V.! (r * sz + c)

    -- Timing in Micro QR starts at position 8 (positions 0-7 are finder/separator).
    -- Position 8: even index → dark (8 is even → dark).
    it "row 0, col 8 is dark (even index, timing)" $
      get 0 8 `shouldBe` True

    it "row 0, col 9 is light (odd index, timing)" $
      get 0 9 `shouldBe` False

    it "col 0, row 8 is dark (even index, timing)" $
      get 8 0 `shouldBe` True

    it "col 0, row 9 is light (odd index, timing)" $
      get 9 0 `shouldBe` False

  -- --------------------------------------------------------------------------
  -- Determinism
  -- --------------------------------------------------------------------------
  describe "determinism" $ do
    it "encode \"12345\" produces the same grid twice" $ do
      let r1 = encode "12345" Nothing Nothing
      let r2 = encode "12345" Nothing Nothing
      r1 `shouldBe` r2

    it "encode \"HELLO\" produces the same grid twice" $ do
      let r1 = encode "HELLO" Nothing Nothing
      let r2 = encode "HELLO" Nothing Nothing
      r1 `shouldBe` r2

  -- --------------------------------------------------------------------------
  -- Auto-selection by input characteristics
  -- --------------------------------------------------------------------------
  describe "auto-selection" $ do
    it "single digit \"1\" → M1 (11×11)" $
      case encode "1" Nothing Nothing of
        Right g -> (mgRows g, mgCols g) `shouldBe` (11, 11)
        Left e  -> expectationFailure (show e)

    it "\"HELLO\" (alphanumeric, 5 chars) → M2 (13×13)" $
      case encode "HELLO" Nothing Nothing of
        Right g -> (mgRows g, mgCols g) `shouldBe` (13, 13)
        Left e  -> expectationFailure (show e)

    it "\"Hello\" (byte mode, mixed case) → M3 or larger" $
      case encode "Hello" Nothing Nothing of
        Right g -> mgRows g `shouldSatisfy` (>= 15)
        Left e  -> expectationFailure (show e)

    it "longer numeric string uses a bigger symbol" $
      case (encode "1" Nothing Nothing, encode "12345678901" Nothing Nothing) of
        (Right g1, Right g2) -> mgRows g1 `shouldSatisfy` (< mgRows g2)
        _                    -> expectationFailure "encode failed"

  -- --------------------------------------------------------------------------
  -- InputTooLong error
  -- --------------------------------------------------------------------------
  describe "InputTooLong" $ do
    it "a 40-digit numeric string returns Left InputTooLong" $
      case encode (replicate 40 '1') Nothing Nothing of
        Left (InputTooLong _) -> return ()
        Left other            -> expectationFailure ("Wrong error: " ++ show other)
        Right _               -> expectationFailure "Expected Left InputTooLong"

    it "a 25-char alphanumeric string returns Left InputTooLong" $
      case encode (replicate 25 'A') Nothing Nothing of
        Left (InputTooLong _) -> return ()
        Left other            -> expectationFailure ("Wrong error: " ++ show other)
        Right _               -> expectationFailure "Expected Left InputTooLong"

  -- --------------------------------------------------------------------------
  -- InvalidConfiguration error
  -- --------------------------------------------------------------------------
  describe "InvalidConfiguration" $ do
    it "M1 + Q returns Left InvalidConfiguration (Q not available for M1)" $
      case encode "1" (Just M1) (Just Q) of
        Left (InvalidConfiguration _) -> return ()
        Left (InputTooLong _)         -> return ()  -- also acceptable
        Left other                    -> expectationFailure ("Wrong error: " ++ show other)
        Right _                       -> expectationFailure "Expected Left error"

  -- --------------------------------------------------------------------------
  -- encodeAt — forced version + ECC
  -- --------------------------------------------------------------------------
  describe "encodeAt" $ do
    it "encodeAt \"1\" M1 Detection → 11×11" $
      case encodeAt "1" M1 Detection of
        Right g -> (mgRows g, mgCols g) `shouldBe` (11, 11)
        Left e  -> expectationFailure (show e)

    it "encodeAt \"1\" M2 L → 13×13" $
      case encodeAt "1" M2 L of
        Right g -> (mgRows g, mgCols g) `shouldBe` (13, 13)
        Left e  -> expectationFailure (show e)

    it "encodeAt \"1\" M3 L → 15×15" $
      case encodeAt "1" M3 L of
        Right g -> (mgRows g, mgCols g) `shouldBe` (15, 15)
        Left e  -> expectationFailure (show e)

    it "encodeAt \"1\" M4 Q → 17×17" $
      case encodeAt "1" M4 Q of
        Right g -> (mgRows g, mgCols g) `shouldBe` (17, 17)
        Left e  -> expectationFailure (show e)

    it "encodeAt returns InputTooLong when data doesn't fit" $
      case encodeAt (replicate 40 '1') M1 Detection of
        Left (InputTooLong _) -> return ()
        Left other            -> expectationFailure ("Wrong error: " ++ show other)
        Right _               -> expectationFailure "Expected Left InputTooLong"

  -- --------------------------------------------------------------------------
  -- ECC level options for M4
  -- --------------------------------------------------------------------------
  describe "ECC levels for M4" $ do
    it "M4/L encodes \"1\" successfully" $
      case encodeAt "1" M4 L of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)

    it "M4/M encodes \"1\" successfully" $
      case encodeAt "1" M4 M of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)

    it "M4/Q encodes \"1\" successfully" $
      case encodeAt "1" M4 Q of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)

    it "M4/L and M4/Q produce different grids (different ECC)" $
      case (encodeAt "1" M4 L, encodeAt "1" M4 Q) of
        (Right g1, Right g2) -> mgModules g1 `shouldNotBe` mgModules g2
        _ -> expectationFailure "encode failed"

  -- --------------------------------------------------------------------------
  -- Module count sanity
  -- --------------------------------------------------------------------------
  describe "module count" $ do
    it "all M1 grids have exactly 121 modules (11×11)" $
      case encode "1" (Just M1) Nothing of
        Right g -> V.length (mgModules g) `shouldBe` 121
        Left e  -> expectationFailure (show e)

    it "all M4 grids have exactly 289 modules (17×17)" $
      case encode "1" (Just M4) Nothing of
        Right g -> V.length (mgModules g) `shouldBe` 289
        Left e  -> expectationFailure (show e)

    it "each grid has at least one dark module" $
      case encode "1" Nothing Nothing of
        Right g -> V.any id (mgModules g) `shouldBe` True
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- Mode selection
  -- --------------------------------------------------------------------------
  describe "mode selection" $ do
    it "pure digits use numeric mode (M1 for 1-5 digits)" $
      case encode "12345" (Just M1) Nothing of
        Right g -> (mgRows g, mgCols g) `shouldBe` (11, 11)
        Left e  -> expectationFailure (show e)

    it "uppercase + digits use alphanumeric mode (M2+ if needed)" $
      case encode "A1B2" Nothing Nothing of
        Right g -> mgRows g `shouldSatisfy` (>= 13)
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- encodeAndLayout round-trip
  -- --------------------------------------------------------------------------
  describe "encodeAndLayout" $ do
    it "returns Right for a simple input" $ do
      let cfg = Barcode2DLayoutConfig
                  { moduleSizePx     = 10
                  , quietZoneModules = 2
                  , foreground       = "#000000"
                  , background       = "#ffffff"
                  }
      case encodeAndLayout "1" Nothing Nothing cfg of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)
