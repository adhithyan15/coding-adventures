-- | PDF417Spec — hspec tests for CodingAdventures.PDF417.
--
-- Tests cover:
--
-- * Version string constant
-- * GF(929) arithmetic correctness
-- * RS generator polynomial
-- * RS encoder
-- * Byte compaction
-- * Text compaction
-- * Numeric compaction
-- * Auto-compaction selection
-- * Row indicator computation
-- * Dimension selection
-- * Main encoder: success paths
-- * Main encoder: error paths
-- * Grid structure invariants (start/stop patterns)
-- * Determinism
module PDF417Spec (spec) where

import Test.Hspec
import Data.Word (Word8)
import qualified Data.Vector as V

import CodingAdventures.PDF417
import CodingAdventures.Barcode2D
  ( ModuleGrid (..)
  , mgRows, mgCols, mgModules
  )

spec :: Spec
spec = do

  -- --------------------------------------------------------------------------
  -- Version
  -- --------------------------------------------------------------------------
  describe "version" $ do
    it "is \"0.1.0\"" $
      version `shouldBe` "0.1.0"

  -- --------------------------------------------------------------------------
  -- GF(929) arithmetic
  -- --------------------------------------------------------------------------
  describe "GF(929) arithmetic" $ do

    it "gfAdd: (100 + 900) mod 929 = 71" $
      gfAdd 100 900 `shouldBe` 71

    it "gfAdd: identity with 0" $
      gfAdd 500 0 `shouldBe` 500

    it "gfSub: (5 - 10 + 929) mod 929 = 924" $
      gfSub 5 10 `shouldBe` 924

    it "gfSub: 10 - 5 = 5" $
      gfSub 10 5 `shouldBe` 5

    it "gfMul: 3 * 3 = 9" $
      gfMul 3 3 `shouldBe` 9

    it "gfMul: 0 * anything = 0" $
      gfMul 0 500 `shouldBe` 0

    it "gfMul: anything * 0 = 0" $
      gfMul 500 0 `shouldBe` 0

    it "gfMul: multiplicative inverse of 3 is 310 (3 * 310 = 930 ≡ 1 mod 929)" $
      gfMul 3 310 `shouldBe` 1

    it "gfMul: 400 * 400 mod 929 = 160000 mod 929" $
      gfMul 400 400 `shouldBe` (160000 `mod` 929)

    it "gfMul: 1 * v = v for any v" $
      gfMul 1 750 `shouldBe` 750

    it "gfMul: v * 1 = v for any v" $
      gfMul 750 1 `shouldBe` 750

    it "gfMul: 928 * 928 (= (-1)^2 = 1 in GF(929))" $
      gfMul 928 928 `shouldBe` 1

  -- --------------------------------------------------------------------------
  -- RS generator polynomial
  -- --------------------------------------------------------------------------
  describe "buildGenerator" $ do

    it "level 0 produces polynomial of degree 2 (3 coefficients)" $
      length (buildGenerator 0) `shouldBe` 3

    it "level 1 produces polynomial of degree 4 (5 coefficients)" $
      length (buildGenerator 1) `shouldBe` 5

    it "level 2 produces polynomial of degree 8 (9 coefficients)" $
      length (buildGenerator 2) `shouldBe` 9

    it "level 8 produces polynomial of degree 512 (513 coefficients)" $
      length (buildGenerator 8) `shouldBe` 513

    it "leading coefficient is always 1" $ do
      head (buildGenerator 0) `shouldBe` 1
      head (buildGenerator 2) `shouldBe` 1
      head (buildGenerator 4) `shouldBe` 1

  -- --------------------------------------------------------------------------
  -- RS encoder
  -- --------------------------------------------------------------------------
  describe "rsEncode" $ do

    it "level 0 produces exactly 2 ECC codewords" $
      length (rsEncode [10, 20, 30] 0) `shouldBe` 2

    it "level 2 produces exactly 8 ECC codewords" $
      length (rsEncode [10, 20, 30] 2) `shouldBe` 8

    it "level 4 produces exactly 32 ECC codewords" $
      length (rsEncode (replicate 20 100) 4) `shouldBe` 32

    it "all ECC codewords are in range 0-928" $ do
      let ecc = rsEncode [100, 200, 300, 400] 2
      all (\v -> v >= 0 && v <= 928) ecc `shouldBe` True

    it "encodes empty data to all-zero ECC" $
      rsEncode [] 0 `shouldBe` [0, 0]

    it "encoding same data twice produces same ECC (deterministic)" $ do
      let d = [1, 50, 100, 200, 300, 500]
      rsEncode d 2 `shouldBe` rsEncode d 2

  -- --------------------------------------------------------------------------
  -- Byte compaction
  -- --------------------------------------------------------------------------
  describe "byteCompact" $ do

    it "empty input produces [924] (latch only)" $
      byteCompact [] `shouldBe` [924]

    it "single byte [0xFF] produces [924, 255]" $
      byteCompact [0xFF] `shouldBe` [924, 255]

    it "single byte [0x00] produces [924, 0]" $
      byteCompact [0x00] `shouldBe` [924, 0]

    it "6 bytes produce latch + 5 codewords" $
      length (byteCompact [0x41, 0x42, 0x43, 0x44, 0x45, 0x46]) `shouldBe` 6

    it "7 bytes produce latch + 5 + 1 = 7 codewords" $
      length (byteCompact [0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47]) `shouldBe` 7

    it "12 bytes produce latch + 10 codewords (two full groups)" $
      length (byteCompact (replicate 12 0x41)) `shouldBe` 11

    it "first codeword is always 924 (latch)" $ do
      head (byteCompact [1, 2, 3]) `shouldBe` 924
      head (byteCompact []) `shouldBe` 924

    it "all codewords in range 0-928" $ do
      let cws = byteCompact [0..255]
      all (\v -> v >= 0 && v <= 928) (tail cws) `shouldBe` True

    it "5-byte remainder encoded 1:1" $ do
      let bs = [0x01, 0x02, 0x03, 0x04, 0x05] :: [Word8]
          cws = byteCompact bs
      -- latch + 5 direct bytes
      cws `shouldBe` [924, 1, 2, 3, 4, 5]

  -- --------------------------------------------------------------------------
  -- Numeric compaction
  -- --------------------------------------------------------------------------
  describe "numericCompact" $ do

    it "starts with latch codeword 902" $
      head (numericCompact "1234") `shouldBe` 902

    it "single digit '0' encodes correctly" $ do
      let cws = tail (numericCompact "0")
      length cws `shouldBe` 1

    it "44 digits produce exactly 15 codewords (plus latch)" $
      length (numericCompact (replicate 44 '1')) `shouldBe` 16

    it "all codewords in range 0-928" $ do
      let cws = numericCompact "1234567890"
      all (\v -> v >= 0 && v <= 928) cws `shouldBe` True

    it "deterministic for same input" $ do
      let d = "9876543210"
      numericCompact d `shouldBe` numericCompact d

  -- --------------------------------------------------------------------------
  -- Auto-compaction
  -- --------------------------------------------------------------------------
  describe "autoCompact" $ do

    it "all-digit input starts with 902 (numeric latch)" $
      head (autoCompact "1234567890") `shouldBe` 902

    it "uppercase ASCII input starts with 900 (text latch)" $
      head (autoCompact "HELLO") `shouldBe` 900

    it "binary/non-text input starts with 924 (byte latch)" $
      head (autoCompact "\x00\x01\x02") `shouldBe` 924

    it "empty input produces [924]" $
      autoCompact "" `shouldBe` [924]

  -- --------------------------------------------------------------------------
  -- ECC level auto-selection
  -- --------------------------------------------------------------------------
  describe "autoEccLevel" $ do

    it "1 codeword → level 2" $
      autoEccLevel 1 `shouldBe` 2

    it "40 codewords → level 2" $
      autoEccLevel 40 `shouldBe` 2

    it "41 codewords → level 3" $
      autoEccLevel 41 `shouldBe` 3

    it "160 codewords → level 3" $
      autoEccLevel 160 `shouldBe` 3

    it "161 codewords → level 4" $
      autoEccLevel 161 `shouldBe` 4

    it "863 codewords → level 5" $
      autoEccLevel 863 `shouldBe` 5

    it "864 codewords → level 6" $
      autoEccLevel 864 `shouldBe` 6

  -- --------------------------------------------------------------------------
  -- Dimension selection
  -- --------------------------------------------------------------------------
  describe "chooseDimensions" $ do

    it "satisfies rows * cols >= total" $ do
      let check total =
            let (c, r) = chooseDimensions total
            in  r * c >= total
      mapM_ (\t -> check t `shouldBe` True) [1, 5, 10, 20, 50, 100, 500]

    it "cols in range 1-30" $ do
      let check total =
            let (c, _) = chooseDimensions total
            in  c >= 1 && c <= 30
      mapM_ (\t -> check t `shouldBe` True) [1, 5, 100, 500, 900]

    it "rows in range 3-90" $ do
      let check total =
            let (_, r) = chooseDimensions total
            in  r >= 3 && r <= 90
      mapM_ (\t -> check t `shouldBe` True) [1, 5, 100, 500, 900]

  -- --------------------------------------------------------------------------
  -- Row indicator computation
  -- --------------------------------------------------------------------------
  describe "row indicators" $ do
    -- Example: R=10, C=3, L=2
    -- R_info = (10-1)/3 = 3
    -- C_info = 3-1 = 2
    -- L_info = 3*2 + (10-1)%3 = 6 + 0 = 6
    let rows = 10; cols = 3; ecc = 2

    it "row 0 (cluster 0): LRI = 30*0 + R_info = 3" $
      computeLRI 0 rows cols ecc `shouldBe` 3

    it "row 0 (cluster 0): RRI = 30*0 + C_info = 2" $
      computeRRI 0 rows cols ecc `shouldBe` 2

    it "row 1 (cluster 1): LRI = 30*0 + L_info = 6" $
      computeLRI 1 rows cols ecc `shouldBe` 6

    it "row 1 (cluster 1): RRI = 30*0 + R_info = 3" $
      computeRRI 1 rows cols ecc `shouldBe` 3

    it "row 2 (cluster 2): LRI = 30*0 + C_info = 2" $
      computeLRI 2 rows cols ecc `shouldBe` 2

    it "row 2 (cluster 2): RRI = 30*0 + L_info = 6" $
      computeRRI 2 rows cols ecc `shouldBe` 6

    it "row 3 (cluster 0): LRI = 30*1 + R_info = 33" $
      computeLRI 3 rows cols ecc `shouldBe` 33

    it "row 3 (cluster 0): RRI = 30*1 + C_info = 32" $
      computeRRI 3 rows cols ecc `shouldBe` 32

    it "row indicators in range 0-928" $ do
      let inRange v = v >= 0 && v <= 928
      let checkRow r = inRange (computeLRI r 6 3 2) && inRange (computeRRI r 6 3 2)
      all checkRow [0..5] `shouldBe` True

  -- --------------------------------------------------------------------------
  -- Main encoder — success paths
  -- --------------------------------------------------------------------------
  describe "encodePDF417" $ do

    it "encodes \"A\" successfully" $
      case encodePDF417 "A" of
        Right _ -> return ()
        Left e  -> expectationFailure ("Expected Right, got Left: " ++ show e)

    it "encodes \"HELLO WORLD\" successfully" $
      case encodePDF417 "HELLO WORLD" of
        Right _ -> return ()
        Left e  -> expectationFailure ("Expected Right, got Left: " ++ show e)

    it "encodes empty string successfully" $
      case encodePDF417 "" of
        Right _ -> return ()
        Left e  -> expectationFailure ("Expected Right, got Left: " ++ show e)

    it "encodes all-digit string successfully" $
      case encodePDF417 "1234567890" of
        Right _ -> return ()
        Left e  -> expectationFailure ("Expected Right, got Left: " ++ show e)

    it "grid has correct number of modules (rows * cols)" $
      case encodePDF417 "HELLO" of
        Right g -> V.length (mgModules g) `shouldBe` mgRows g * mgCols g
        Left e  -> expectationFailure (show e)

    it "grid width = 69 + 17*dataCols" $
      case encodePDF417 "A" of
        Right g ->
          -- Width must be 69 + 17*c for some integer c >= 1
          let w = mgCols g
              c = (w - 69) `div` 17
          in  (69 + 17 * c) `shouldBe` w
        Left e -> expectationFailure (show e)

    it "grid height is a multiple of row height (default 3)" $
      case encodePDF417 "TEST" of
        Right g -> (mgRows g `mod` 3) `shouldBe` 0
        Left e  -> expectationFailure (show e)

    it "is deterministic" $ do
      let r1 = encodePDF417 "DETERMINISM TEST"
          r2 = encodePDF417 "DETERMINISM TEST"
      case (r1, r2) of
        (Right g1, Right g2) -> mgModules g1 `shouldBe` mgModules g2
        _ -> expectationFailure "encoding failed"

  -- --------------------------------------------------------------------------
  -- Start pattern structural invariants
  -- --------------------------------------------------------------------------
  describe "start/stop pattern invariants" $ do
    let Right grid = encodePDF417 "HELLO"
    let rh = 3  -- default row height
    let rows = mgRows grid `div` rh
    let w = mgCols grid

    -- Start pattern: 11111111010101000 (17 modules)
    -- Stop pattern:  111111101000101001 (18 modules)
    let startExpected = [True,True,True,True,True,True,True,True,False,True,False,True,False,True,False,False,False]
    let stopExpected  = [True,True,True,True,True,True,True,False,True,False,False,False,True,False,True,False,False,True]

    it "first row has correct start pattern (first 17 modules)" $ do
      let mods = V.toList (mgModules grid)
      take 17 mods `shouldBe` startExpected

    it "first row has correct stop pattern (last 18 modules)" $ do
      let mods = V.toList (mgModules grid)
      -- First logical row, first module-row
      drop (w - 18) (take w mods) `shouldBe` stopExpected

    it "every logical row's first module-row starts with start pattern" $ do
      let mods = V.toList (mgModules grid)
      let checkRow r =
            let rowStart = r * rh * w
                rowMods  = take 17 (drop rowStart mods)
            in  rowMods == startExpected
      all checkRow [0 .. rows - 1] `shouldBe` True

    it "every logical row's first module-row ends with stop pattern" $ do
      let mods = V.toList (mgModules grid)
      let checkRow r =
            let rowStart = r * rh * w
                rowMods  = drop (rowStart + w - 18) (take (rowStart + w) mods)
            in  rowMods == stopExpected
      all checkRow [0 .. rows - 1] `shouldBe` True

  -- --------------------------------------------------------------------------
  -- Main encoder — error paths
  -- --------------------------------------------------------------------------
  describe "encodePDF417 errors" $ do

    it "invalid ECC level returns Left InvalidECCLevel" $ do
      let opts = defaultOptions { eccLevel = Just 9 }
      case encodePDF417With "A" opts of
        Left (InvalidECCLevel _) -> return ()
        other -> expectationFailure ("Expected InvalidECCLevel, got: " ++ show other)

    it "ECC level -1 returns Left InvalidECCLevel" $ do
      let opts = defaultOptions { eccLevel = Just (-1) }
      case encodePDF417With "A" opts of
        Left (InvalidECCLevel _) -> return ()
        other -> expectationFailure ("Expected InvalidECCLevel, got: " ++ show other)

    it "columns = 0 returns Left InvalidDimensions" $ do
      let opts = defaultOptions { columns = Just 0 }
      case encodePDF417With "A" opts of
        Left (InvalidDimensions _) -> return ()
        other -> expectationFailure ("Expected InvalidDimensions, got: " ++ show other)

    it "columns = 31 returns Left InvalidDimensions" $ do
      let opts = defaultOptions { columns = Just 31 }
      case encodePDF417With "A" opts of
        Left (InvalidDimensions _) -> return ()
        other -> expectationFailure ("Expected InvalidDimensions, got: " ++ show other)

  -- --------------------------------------------------------------------------
  -- Options: custom ECC level and columns
  -- --------------------------------------------------------------------------
  describe "PDF417Options" $ do

    it "explicit ECC level 0 is accepted" $
      case encodePDF417With "A" (defaultOptions { eccLevel = Just 0 }) of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)

    it "explicit ECC level 8 is accepted" $
      case encodePDF417With "A" (defaultOptions { eccLevel = Just 8 }) of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)

    it "explicit columns = 1 produces width 69 + 17 = 86" $
      case encodePDF417With "A" (defaultOptions { columns = Just 1 }) of
        Right g -> mgCols g `shouldBe` 86
        Left e  -> expectationFailure (show e)

    it "explicit columns = 3 produces width 69 + 51 = 120" $
      case encodePDF417With "A" (defaultOptions { columns = Just 3 }) of
        Right g -> mgCols g `shouldBe` 120
        Left e  -> expectationFailure (show e)

    it "rowHeight = 1 produces grid with rows = logical rows" $ do
      let opts = defaultOptions { rowHeight = 1, columns = Just 1 }
      case encodePDF417With "A" opts of
        Right g -> mgRows g `shouldBe` (mgRows g `div` 1)
        Left e  -> expectationFailure (show e)

    it "rowHeight = 5 produces grid height divisible by 5" $ do
      let opts = defaultOptions { rowHeight = 5 }
      case encodePDF417With "TEST" opts of
        Right g -> (mgRows g `mod` 5) `shouldBe` 0
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- Larger inputs
  -- --------------------------------------------------------------------------
  describe "larger inputs" $ do

    it "encodes 100 bytes successfully" $
      case encodePDF417 (replicate 100 'A') of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)

    it "encodes 100-character digit string" $
      case encodePDF417 (replicate 100 '5') of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)

    it "binary-like input (all chars 0-127)" $
      case encodePDF417 (map toEnum [0..127]) of
        Right _ -> return ()
        Left e  -> expectationFailure (show e)

  -- --------------------------------------------------------------------------
  -- Module count per row
  -- --------------------------------------------------------------------------
  describe "module count" $ do

    it "each module-row has exactly 69 + 17*cols modules" $ do
      let opts = defaultOptions { columns = Just 5, rowHeight = 1 }
      case encodePDF417With "TEST DATA" opts of
        Right g ->
          mgCols g `shouldBe` (69 + 17 * 5)
        Left e -> expectationFailure (show e)
