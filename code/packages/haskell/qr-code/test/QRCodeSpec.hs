-- | Unit and integration tests for QRCode.
--
-- Test coverage:
--   1.  ECC level indicators (eccIndicator mapping)
--   2.  Symbol size formula: (4V+17)
--   3.  numRawDataModules — spot-check v1 and v7
--   4.  numDataCodewords — spot-check known values from ISO table
--   5.  buildGenerator — length and monic check for several sizes
--   6.  rsEncode — ECC codeword count
--   7.  selectMode — numeric / alphanumeric / byte selection
--   8.  buildDataCodewords — length equals numDataCodewords
--   9.  computeBlocks — correct block count
--   10. interleaveBlocks — correct total codeword count
--   11. computeFormatBits — BCH validity check
--   12. computeVersionBits — v7 and v10 encode correct version number
--   13. encode "HELLO WORLD" M → 21×21 grid
--   14. encode "https://example.com" M → 25×25 grid
--   15. encode single char "A" M → 21×21 grid
--   16. encode empty string M → 21×21 grid
--   17. all four ECC levels produce valid grids
--   18. H needs larger version than L for same long input
--   19. finder pattern at (0,0) in encoded symbol
--   20. finder pattern at (0, sz-7) in encoded symbol
--   21. finder pattern at (sz-7, 0) in encoded symbol
--   22. timing strip row 6 alternates correctly
--   23. timing strip col 6 alternates correctly
--   24. dark module at (4V+9, 8) is dark
--   25. format information is BCH-valid
--   26. ECC indicator bits match expected values for all four levels
--   27. format info copy 1 bits match copy 2 bits
--   28. deterministic: same input → identical grid
--   29. different inputs → different grids
--   30. InputTooLong error for giant input
--   31. numeric mode produces smallest grid for digit-only input
--   32. test corpus: 5 canonical inputs all encode to valid grids
--   33. grid is square (rows == cols)
--   34. grid size = 4*version+17
--   35. version 7+ encoded when short input at H is padded to v7
module QRCodeSpec (spec) where

import Test.Hspec
import qualified Data.Vector as V
import Data.Bits (xor, shiftR, shiftL, (.&.), (.|.))

import QRCode
import CodingAdventures.Barcode2D (ModuleGrid (..), mgRows, mgCols, mgModules)

-- ---------------------------------------------------------------------------
-- Helpers
-- ---------------------------------------------------------------------------

-- | Get the module value at (row, col) in a ModuleGrid.
getModule :: ModuleGrid -> Int -> Int -> Bool
getModule grid row col =
  mgModules grid V.! (row * mgCols grid + col)

-- | Verify a 7×7 finder pattern at (top, left) in the encoded grid.
hasFinder :: ModuleGrid -> Int -> Int -> Bool
hasFinder grid top left =
  all (\(dr, dc) ->
    let onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6
        inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
        expected = onBorder || inCore
    in  getModule grid (top + dr) (left + dc) == expected
  ) [(dr, dc) | dr <- [0..6], dc <- [0..6]]

-- | Read the 15-bit format info from copy 1 and BCH-verify it.
-- Returns (ecc_bits, mask_bits) on success, Nothing on BCH failure.
formatInfoValid :: ModuleGrid -> Maybe (Int, Int)
formatInfoValid grid =
  let -- Copy 1 positions ordered f14..f0
      positions :: [(Int, Int)]
      positions =
        [(8, 0),(8, 1),(8, 2),(8, 3),(8, 4),(8, 5),(8, 7),(8, 8)
        ,(7, 8),(5, 8),(4, 8),(3, 8),(2, 8),(1, 8),(0, 8)
        ]
      -- Extract raw 15-bit word (f14 at index 0, f0 at index 14)
      raw = foldr (\(i, (r, c)) acc ->
                let bit = if getModule grid r c then 1 else 0
                in  acc .|. (bit `shiftL` (14 - i)))
              0 (zip [0..14] positions)
      -- Remove ISO XOR mask
      fmt = raw `xor` 0x5412
      -- BCH check: recompute 10-bit remainder from 5-bit data portion
      dataWord = fmt `shiftR` 10
      rem0 = dataWord `shiftL` 10
      rem' = foldl (\r i -> if (r `shiftR` i) .&. 1 == 1
                             then r `xor` (0x537 `shiftL` (i - 10))
                             else r)
               rem0 [14, 13 .. 10]
  in  if (rem' .&. 0x3FF) /= (fmt .&. 0x3FF)
      then Nothing
      else Just ((fmt `shiftR` 13) .&. 0x3, (fmt `shiftR` 10) .&. 0x7)

-- ---------------------------------------------------------------------------
-- Spec
-- ---------------------------------------------------------------------------

spec :: Spec
spec = do

  -- -------------------------------------------------------------------------
  -- 1. ECC level indicators
  -- -------------------------------------------------------------------------
  describe "ECC level indicator mapping" $ do

    it "L maps to 01" $ eccIndicator L `shouldBe` 1
    it "M maps to 00" $ eccIndicator M `shouldBe` 0
    it "Q maps to 11" $ eccIndicator Q `shouldBe` 3
    it "H maps to 10" $ eccIndicator H `shouldBe` 2

  -- -------------------------------------------------------------------------
  -- 2. Symbol size formula
  -- -------------------------------------------------------------------------
  describe "symbolSize" $ do

    it "version 1 → 21" $ symbolSize 1 `shouldBe` 21
    it "version 2 → 25" $ symbolSize 2 `shouldBe` 25
    it "version 7 → 45" $ symbolSize 7 `shouldBe` 45
    it "version 10 → 57" $ symbolSize 10 `shouldBe` 57
    it "version 40 → 177" $ symbolSize 40 `shouldBe` 177

  -- -------------------------------------------------------------------------
  -- 3. numRawDataModules spot-check
  -- -------------------------------------------------------------------------
  describe "numRawDataModules" $ do

    it "v1 = 208" $ numRawDataModules 1 `shouldBe` 208
    it "v7 = 1568 (includes alignment + version info)" $
      numRawDataModules 7 `shouldBe` 1568

  -- -------------------------------------------------------------------------
  -- 4. numDataCodewords spot-check
  -- -------------------------------------------------------------------------
  describe "numDataCodewords" $ do

    it "v1 L = 19"  $ numDataCodewords 1 L `shouldBe` 19
    it "v1 M = 16"  $ numDataCodewords 1 M `shouldBe` 16
    it "v1 Q = 13"  $ numDataCodewords 1 Q `shouldBe` 13
    it "v1 H = 9"   $ numDataCodewords 1 H `shouldBe` 9
    it "v2 L = 34"  $ numDataCodewords 2 L `shouldBe` 34

  -- -------------------------------------------------------------------------
  -- 5. buildGenerator
  -- -------------------------------------------------------------------------
  describe "buildGenerator" $ do

    it "generator of degree 7 has length 8" $
      length (buildGenerator 7) `shouldBe` 8

    it "generator is monic (first coefficient = 1)" $
      head (buildGenerator 7) `shouldBe` 1

    it "generator of degree 10 has length 11" $
      length (buildGenerator 10) `shouldBe` 11

    it "generator of degree 17 has length 18" $
      length (buildGenerator 17) `shouldBe` 18

  -- -------------------------------------------------------------------------
  -- 6. rsEncode
  -- -------------------------------------------------------------------------
  describe "rsEncode" $ do

    it "produces exactly n ECC codewords for degree-n generator" $
      let gen = buildGenerator 10
          ecc = rsEncode [32, 91, 11, 120, 209, 114, 220, 77, 67, 64, 236, 17, 236] gen
      in  length ecc `shouldBe` 10

    it "ECC bytes are in range 0..255" $
      let gen = buildGenerator 7
          ecc = rsEncode [16, 32, 12, 86, 97, 128, 236, 17, 236] gen
      in  all (\b -> b >= 0 && b <= 255) ecc `shouldBe` True

  -- -------------------------------------------------------------------------
  -- 7. selectMode
  -- -------------------------------------------------------------------------
  describe "selectMode" $ do

    it "all-digit input selects Numeric" $
      selectMode "01234567" `shouldBe` Numeric

    it "empty string selects Numeric" $
      selectMode "" `shouldBe` Numeric

    it "uppercase + space selects Alphanumeric" $
      selectMode "HELLO WORLD" `shouldBe` Alphanumeric

    it "lowercase input selects Byte" $
      selectMode "hello" `shouldBe` Byte

    it "URL selects Byte" $
      selectMode "https://example.com" `shouldBe` Byte

    it "alphanumeric special chars stay in Alphanumeric" $
      selectMode "A$B%C*D+E-F.G/H:I" `shouldBe` Alphanumeric

  -- -------------------------------------------------------------------------
  -- 8. buildDataCodewords length
  -- -------------------------------------------------------------------------
  describe "buildDataCodewords length" $ do

    it "length equals numDataCodewords for 'HELLO WORLD' v1 M" $
      length (buildDataCodewords "HELLO WORLD" 1 M)
        `shouldBe` numDataCodewords 1 M

    it "length equals numDataCodewords for 'https://example.com' v2 M" $
      length (buildDataCodewords "https://example.com" 2 M)
        `shouldBe` numDataCodewords 2 M

  -- -------------------------------------------------------------------------
  -- 9. computeBlocks
  -- -------------------------------------------------------------------------
  describe "computeBlocks" $ do

    it "v1 M produces 1 block" $
      let cws = buildDataCodewords "HELLO WORLD" 1 M
      in  length (computeBlocks cws 1 M) `shouldBe` 1

    it "v5 Q produces 4 blocks" $
      let cws = replicate (numDataCodewords 5 Q) 0
      in  length (computeBlocks cws 5 Q) `shouldBe` 4

  -- -------------------------------------------------------------------------
  -- 10. interleaveBlocks
  -- -------------------------------------------------------------------------
  describe "interleaveBlocks" $ do

    it "interleaved total length = data codewords + ECC codewords" $
      let cws    = buildDataCodewords "HELLO WORLD" 1 M
          blocks = computeBlocks cws 1 M
          result = interleaveBlocks blocks
          eIdx   = 1  -- M
          totalEcc = (numBlocks !! eIdx !! 1) * (eccCwPerBlock !! eIdx !! 1)
      in  length result `shouldBe` numDataCodewords 1 M + totalEcc

  -- -------------------------------------------------------------------------
  -- 11. computeFormatBits — BCH validity
  -- -------------------------------------------------------------------------
  describe "computeFormatBits" $ do

    it "format bits for (M, mask=2) pass BCH check" $
      let fmt    = computeFormatBits M 2
          raw    = fmt `xor` 0x5412
          dataBits = raw `shiftR` 10
          rem0   = dataBits `shiftL` 10
          rem'   = foldl (\r i -> if (r `shiftR` i) .&. 1 == 1
                                  then r `xor` (0x537 `shiftL` (i - 10))
                                  else r)
                     rem0 [14, 13 .. 10]
      in  (rem' .&. 0x3FF) `shouldBe` (raw .&. 0x3FF)

    it "format bits for (L, mask=0) contain ECC indicator 01 in bits 13-12" $
      let fmt = computeFormatBits L 0
          raw = fmt `xor` 0x5412
      in  (raw `shiftR` 13) .&. 0x3 `shouldBe` 1  -- L = 01

  -- -------------------------------------------------------------------------
  -- 12. computeVersionBits
  -- -------------------------------------------------------------------------
  describe "computeVersionBits" $ do

    it "v7 version bits encode version 7 in bits 17-12" $
      computeVersionBits 7 `shiftR` 12 `shouldBe` 7

    it "v10 version bits encode version 10 in bits 17-12" $
      computeVersionBits 10 `shiftR` 12 `shouldBe` 10

  -- -------------------------------------------------------------------------
  -- 13. encode "HELLO WORLD" M
  -- -------------------------------------------------------------------------
  describe "encode HELLO WORLD M" $ do

    let grid = case encode "HELLO WORLD" M of
                 Right g -> g
                 Left e  -> error (show e)

    it "produces 21×21 grid (version 1)" $ do
      mgRows grid `shouldBe` 21
      mgCols grid `shouldBe` 21

    it "grid is square" $
      mgRows grid `shouldBe` mgCols grid

    it "finder pattern at (0,0)" $
      hasFinder grid 0 0 `shouldBe` True

    it "finder pattern at (0, size-7)" $
      hasFinder grid 0 (mgCols grid - 7) `shouldBe` True

    it "finder pattern at (size-7, 0)" $
      hasFinder grid (mgRows grid - 7) 0 `shouldBe` True

  -- -------------------------------------------------------------------------
  -- 14. encode "https://example.com" M
  -- -------------------------------------------------------------------------
  describe "encode https://example.com M" $ do

    let grid = case encode "https://example.com" M of
                 Right g -> g
                 Left e  -> error (show e)

    it "produces 25×25 grid (version 2)" $
      mgRows grid `shouldBe` 25

    it "dark module at (4*2+9, 8) = (17, 8)" $
      getModule grid 17 8 `shouldBe` True

  -- -------------------------------------------------------------------------
  -- 15. encode single char "A" M
  -- -------------------------------------------------------------------------
  describe "encode single char A M" $ do

    it "produces 21×21 grid" $ do
      let Right grid = encode "A" M
      mgRows grid `shouldBe` 21

    it "dark module at (13, 8) for version 1" $ do
      let Right grid = encode "A" M
      getModule grid 13 8 `shouldBe` True  -- 4*1+9 = 13

  -- -------------------------------------------------------------------------
  -- 16. encode empty string M
  -- -------------------------------------------------------------------------
  describe "encode empty string M" $ do

    it "produces 21×21 grid" $ do
      let Right grid = encode "" M
      mgRows grid `shouldBe` 21

  -- -------------------------------------------------------------------------
  -- 17. all four ECC levels produce valid grids
  -- -------------------------------------------------------------------------
  describe "all ECC levels" $ do

    it "L produces valid grid" $ do
      let Right g = encode "HELLO" L
      mgRows g `shouldSatisfy` (>= 21)

    it "M produces valid grid" $ do
      let Right g = encode "HELLO" M
      mgRows g `shouldSatisfy` (>= 21)

    it "Q produces valid grid" $ do
      let Right g = encode "HELLO" Q
      mgRows g `shouldSatisfy` (>= 21)

    it "H produces valid grid" $ do
      let Right g = encode "HELLO" H
      mgRows g `shouldSatisfy` (>= 21)

  -- -------------------------------------------------------------------------
  -- 18. H needs at least as large a version as L
  -- -------------------------------------------------------------------------
  describe "ECC level capacity ordering" $ do

    it "H needs larger or equal version than L for longer input" $ do
      let Right gL = encode "The quick brown fox" L
      let Right gH = encode "The quick brown fox" H
      mgRows gH `shouldSatisfy` (>= mgRows gL)

  -- -------------------------------------------------------------------------
  -- 19-21. Finder patterns in URL encoding
  -- -------------------------------------------------------------------------
  describe "finder patterns in URL encoding" $ do

    let Right grid = encode "https://example.com" M

    it "finder at (0,0)" $
      hasFinder grid 0 0 `shouldBe` True

    it "finder at (0, sz-7)" $
      hasFinder grid 0 (mgCols grid - 7) `shouldBe` True

    it "finder at (sz-7, 0)" $
      hasFinder grid (mgRows grid - 7) 0 `shouldBe` True

  -- -------------------------------------------------------------------------
  -- 22-23. Timing strips
  -- -------------------------------------------------------------------------
  describe "timing strips" $ do

    let Right grid = encode "HELLO WORLD" M
    let sz = mgRows grid

    it "timing row 6 alternates dark/light from col 8 to sz-9" $
      all (\c -> getModule grid 6 c == even c) [8..sz-9] `shouldBe` True

    it "timing col 6 alternates dark/light from row 8 to sz-9" $
      all (\r -> getModule grid r 6 == even r) [8..sz-9] `shouldBe` True

  -- -------------------------------------------------------------------------
  -- 24. Dark module
  -- -------------------------------------------------------------------------
  describe "dark module" $ do

    it "dark module at (13, 8) for version 1" $ do
      let Right g = encode "HELLO WORLD" M
      getModule g 13 8 `shouldBe` True  -- 4*1+9 = 13

    it "dark module at (17, 8) for version 2" $ do
      let Right g = encode "https://example.com" M
      getModule g 17 8 `shouldBe` True  -- 4*2+9 = 17

  -- -------------------------------------------------------------------------
  -- 25-26. Format information
  -- -------------------------------------------------------------------------
  describe "format information" $ do

    it "format info is BCH-valid for HELLO WORLD M" $ do
      let Right g = encode "HELLO WORLD" M
      formatInfoValid g `shouldSatisfy` (/= Nothing)

    it "ECC indicator bits are correct for M (00)" $ do
      let Right g = encode "HELLO WORLD" M
      fmap fst (formatInfoValid g) `shouldBe` Just 0  -- M = 00

    it "ECC indicator bits are correct for L (01)" $ do
      let Right g = encode "HELLO" L
      fmap fst (formatInfoValid g) `shouldBe` Just 1  -- L = 01

    it "ECC indicator bits are correct for Q (11)" $ do
      let Right g = encode "HELLO" Q
      fmap fst (formatInfoValid g) `shouldBe` Just 3  -- Q = 11

    it "ECC indicator bits are correct for H (10)" $ do
      let Right g = encode "HELLO" H
      fmap fst (formatInfoValid g) `shouldBe` Just 2  -- H = 10

  -- -------------------------------------------------------------------------
  -- 27. Format info copies match
  -- -------------------------------------------------------------------------
  describe "format info copies match" $ do

    it "copy 1 and copy 2 carry the same bits for HELLO WORLD M" $ do
      let Right g = encode "HELLO WORLD" M
      let sz = mgRows g
      let copy1Pos :: [(Int, Int)]
          copy1Pos = [(8,0),(8,1),(8,2),(8,3),(8,4),(8,5),(8,7),(8,8)
                     ,(7,8),(5,8),(4,8),(3,8),(2,8),(1,8),(0,8)]
      let copy2Pos :: [(Int, Int)]
          copy2Pos = [(sz-1,8),(sz-2,8),(sz-3,8),(sz-4,8),(sz-5,8),(sz-6,8),(sz-7,8)
                     ,(8,sz-8),(8,sz-7),(8,sz-6),(8,sz-5),(8,sz-4),(8,sz-3),(8,sz-2),(8,sz-1)]
      let bits1 = map (\(r,c) -> getModule g r c) copy1Pos
      let bits2 = map (\(r,c) -> getModule g r c) copy2Pos
      bits1 `shouldBe` bits2

  -- -------------------------------------------------------------------------
  -- 28. Deterministic encoding
  -- -------------------------------------------------------------------------
  describe "deterministic" $ do

    it "same input produces identical grids" $ do
      let Right g1 = encode "https://example.com" M
      let Right g2 = encode "https://example.com" M
      mgModules g1 `shouldBe` mgModules g2

  -- -------------------------------------------------------------------------
  -- 29. Different inputs differ
  -- -------------------------------------------------------------------------
  describe "different inputs" $ do

    it "HELLO and WORLD produce different grids" $ do
      let Right g1 = encode "HELLO" M
      let Right g2 = encode "WORLD" M
      let sz = mgRows g1
      let differ = or [ getModule g1 r c /= getModule g2 r c
                      | r <- [0..sz-1], c <- [0..sz-1] ]
      differ `shouldBe` True

  -- -------------------------------------------------------------------------
  -- 30. InputTooLong error
  -- -------------------------------------------------------------------------
  describe "InputTooLong error" $ do

    it "returns InputTooLong for 8000-char input at H" $
      let giant = replicate 8000 'A'
      in  case encode giant H of
            Left (InputTooLong _) -> True `shouldBe` True
            _                     -> expectationFailure "expected InputTooLong"

  -- -------------------------------------------------------------------------
  -- 31. Numeric mode uses smallest version
  -- -------------------------------------------------------------------------
  describe "numeric mode" $ do

    it "15 digits fit in version 1 at M" $ do
      let Right g = encode "000000000000000" M
      mgRows g `shouldBe` 21

  -- -------------------------------------------------------------------------
  -- 32. Test corpus
  -- -------------------------------------------------------------------------
  describe "test corpus" $ do

    let corpus =
          [ ("A",                                         M)
          , ("HELLO WORLD",                               M)
          , ("https://example.com",                       M)
          , ("01234567890",                               M)
          , ("The quick brown fox jumps over the lazy dog", M)
          ]

    mapM_ (\(input, ecc) ->
      it ("encodes: " ++ take 40 input) $ do
        case encode input ecc of
          Left err -> fail (show err)
          Right g  -> do
            mgRows g `shouldSatisfy` (>= 21)
            mgRows g `shouldBe` mgCols g
            formatInfoValid g `shouldSatisfy` (/= Nothing)
      ) corpus

  -- -------------------------------------------------------------------------
  -- 33. Grid is square
  -- -------------------------------------------------------------------------
  describe "grid dimensions" $ do

    it "all corpus inputs produce square grids" $
      all (\input ->
        case encode input M of
          Right g -> mgRows g == mgCols g
          Left _  -> False)
      ["A", "HELLO WORLD", "https://example.com"] `shouldBe` True

  -- -------------------------------------------------------------------------
  -- 34. Grid size = 4*version + 17
  -- -------------------------------------------------------------------------
  describe "grid size formula" $ do

    it "v1 grid: rows = 4*1+17 = 21" $ do
      let Right g = encode "A" M
      mgRows g `shouldBe` 21

    it "v2 grid: rows = 4*2+17 = 25" $ do
      let Right g = encode "https://example.com" M
      mgRows g `shouldBe` 25

  -- -------------------------------------------------------------------------
  -- 35. Version 7+ encoding
  -- -------------------------------------------------------------------------
  describe "version 7+ encoding" $ do

    it "85 uppercase chars at H produce version >= 7 (grid >= 45×45)" $ do
      let input = replicate 85 'A'
      case encode input H of
        Right g -> mgRows g `shouldSatisfy` (>= 45)
        Left e  -> fail (show e)

    it "dark module is correct for any version >= 7" $ do
      let input = replicate 85 'A'
      case encode input H of
        Right g ->
          let sz = mgRows g
              v  = (sz - 17) `div` 4
          in  getModule g (4 * v + 9) 8 `shouldBe` True
        Left e -> fail (show e)
