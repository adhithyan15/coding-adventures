module CtCompareSpec (spec) where

import Control.Monad (forM_)
import Data.Bits (shiftL, xor)
import qualified Data.ByteString as B
import Data.Word (Word64, Word8)
import Test.Hspec

import Corpus
import CtCompare

-- | Corpus values are public test data, so mapping a boolean to a bit with an
-- ordinary conditional is fine here (adapters are test-only code).
bitOf :: Bool -> Word8
bitOf b = if b then 1 else 0

bs :: [Word8] -> B.ByteString
bs = B.pack

spec :: Spec
spec = do
  corpusSpec
  equalitySpec
  selectionSpec
  u64Spec

-- ---------------------------------------------------------------------------
-- Shared CT01 corpus
-- ---------------------------------------------------------------------------

corpusSpec :: Spec
corpusSpec = describe "CT01 corpus" $ do
  (profile, cases) <- runIO loadCorpus
  it "is the ct-compare-v1 profile with 21 cases" $ do
    profile `shouldBe` "ct-compare-v1"
    length cases `shouldBe` 21
  forM_ cases $ \c -> it (caseId c) (runCase c)

field :: Case -> String -> Json
field c k = maybe (error ("missing field " ++ k)) id (lookup k (caseFields c))

hexField :: Case -> String -> B.ByteString
hexField c k = case field c k of
  JString s -> hexToBytes s
  _ -> error (k ++ " is not a string")

expectedField :: Case -> String -> Maybe Json
expectedField c k = case field c "expected" of
  JObject fs -> lookup k fs
  _ -> Nothing

runCase :: Case -> Expectation
runCase c = case caseOp c of
  "eq" -> do
    let l = hexField c "left_hex"
        r = hexField c "right_hex"
        want = expectedEqual c
    ctEq l r `shouldBe` want
    ctEqBit l r `shouldBe` bitOf want
  "eq_fixed" -> do
    let l = hexField c "left_hex"
        r = hexField c "right_hex"
        want = expectedEqual c
    case field c "fixed_size_bytes" of
      JInt n -> fromIntegral (B.length l) `shouldBe` n
      _ -> expectationFailure "fixed_size_bytes missing"
    ctEqFixed l r `shouldBe` want
    ctEqFixedBit l r `shouldBe` bitOf want
  "select_bytes" -> do
    let l = hexField c "left_hex"
        r = hexField c "right_hex"
        choice = case field c "choice" of
          JBool b -> bitOf b
          _ -> error "choice missing"
    case expectedField c "error_id" of
      Just (JString e) -> do
        e `shouldBe` "CT_SELECT_LENGTH_MISMATCH"
        ctSelectBytes l r choice `shouldBe` Left SelectLengthMismatch
      _ -> case expectedField c "output_hex" of
        Just (JString h) -> ctSelectBytes l r choice `shouldBe` Right (hexToBytes h)
        _ -> expectationFailure "select case has neither error nor output"
  "eq_u64" -> do
    let w k = case field c k of
          JString s -> hexToWord64 s
          _ -> error "u64 field missing"
        l = w "left_u64"
        r = w "right_u64"
        want = expectedEqual c
    ctEqU64 l r `shouldBe` want
    ctEqU64Bit l r `shouldBe` bitOf want
  other -> expectationFailure ("unknown corpus operation " ++ other)

expectedEqual :: Case -> Bool
expectedEqual c = case expectedField c "equal" of
  Just (JBool b) -> b
  _ -> error "expected.equal missing"

-- ---------------------------------------------------------------------------
-- Equality beyond the corpus
-- ---------------------------------------------------------------------------

equalitySpec :: Spec
equalitySpec = describe "ctEq" $ do
  it "accepts equal and empty strings" $ do
    ctEq B.empty B.empty `shouldBe` True
    ctEq (bs [1, 2, 3]) (bs [1, 2, 3]) `shouldBe` True
    ctEqBit (bs [9]) (bs [9]) `shouldBe` 1
    ctEqFixedBit B.empty B.empty `shouldBe` 1

  it "detects a mismatch at every byte position of 64 bytes" $ do
    let base = [fromIntegral ((i * 7) `mod` 256) | i <- [1 .. 64 :: Int]] :: [Word8]
        a = B.pack base
    forM_ [0 .. 63 :: Int] $ \pos -> do
      let b = B.pack [if j == pos then x `xor` 1 else x | (j, x) <- zip [0 ..] base]
      ctEq a b `shouldBe` False
      ctEqBit a b `shouldBe` 0
      ctEqFixed a b `shouldBe` False

  it "detects every single-bit difference of every byte value" $
    forM_ [0 .. 255 :: Word8] $ \v -> do
      ctEq (bs [v]) (bs [v]) `shouldBe` True
      forM_ [0 .. 7 :: Int] $ \bit ->
        ctEq (bs [v]) (bs [v `xor` (1 `shiftL` bit)]) `shouldBe` False

  it "treats every non-zero accumulator, 1..255, as different" $
    -- accumulator values come from a single differing byte against zero
    forM_ [1 .. 255 :: Word8] $ \v ->
      ctEqBit (bs [0, 0]) (bs [0, v]) `shouldBe` 0

  it "treats a length mismatch as false in both directions" $ do
    ctEq (bs [1]) (bs [1, 2]) `shouldBe` False
    ctEq (bs [1, 2]) (bs [1]) `shouldBe` False
    ctEq B.empty (bs [0]) `shouldBe` False
    ctEqBit B.empty (bs [0]) `shouldBe` 0
    ctEqFixed (bs [1, 2, 3]) (bs [1, 2, 3, 4]) `shouldBe` False

  it "treats NUL and high-bit bytes as data" $ do
    ctEq (bs [0x61, 0, 0x62]) (bs [0x61, 0, 0x63]) `shouldBe` False
    ctEq (bs [0xff, 0x80]) (bs [0xff, 0x80]) `shouldBe` True
    ctEq (bs [0xff]) (bs [0x7f]) `shouldBe` False

-- ---------------------------------------------------------------------------
-- Selection
-- ---------------------------------------------------------------------------

selectionSpec :: Spec
selectionSpec = describe "ctSelectBytes" $ do
  it "picks left for 1 and right for 0" $ do
    ctSelectBytes (bs [65, 66]) (bs [67, 68]) 1 `shouldBe` Right (bs [65, 66])
    ctSelectBytes (bs [65, 66]) (bs [67, 68]) 0 `shouldBe` Right (bs [67, 68])

  it "is correct for all 256 byte values against all 256 byte values" $ do
    let right = B.pack [0 .. 255]
    forM_ [0 .. 255 :: Word8] $ \lv -> do
      let left = B.replicate 256 lv
      ctSelectBytes left right 1 `shouldBe` Right left
      ctSelectBytes left right 0 `shouldBe` Right right

  it "handles empty inputs" $ do
    ctSelectBytes B.empty B.empty 1 `shouldBe` Right B.empty
    ctSelectBytes B.empty B.empty 0 `shouldBe` Right B.empty

  it "reports SelectLengthMismatch for unequal lengths" $ do
    ctSelectBytes (bs [1]) (bs [1, 2]) 1 `shouldBe` Left SelectLengthMismatch
    ctSelectBytes (bs [1, 2]) (bs [1]) 0 `shouldBe` Left SelectLengthMismatch
    ctSelectBytes B.empty (bs [1]) 1 `shouldBe` Left SelectLengthMismatch

  it "checks lengths before the choice" $
    ctSelectBytes (bs [1]) (bs [1, 2]) 7 `shouldBe` Left SelectLengthMismatch

  it "rejects every choice other than 0 and 1" $
    forM_ [2 .. 255 :: Word8] $ \choice ->
      ctSelectBytes (bs [1]) (bs [2]) choice `shouldBe` Left InvalidChoice

  it "composes with the bit-returning equalities" $ do
    ctSelectBytes (bs [1]) (bs [2]) (ctEqBit (bs [7]) (bs [7])) `shouldBe` Right (bs [1])
    ctSelectBytes (bs [1]) (bs [2]) (ctEqBit (bs [7]) (bs [8])) `shouldBe` Right (bs [2])
    ctSelectBytes (bs [1]) (bs [2]) (ctEqU64Bit 5 6) `shouldBe` Right (bs [2])

  it "returns fresh storage that does not alias an input" $ do
    let l = B.pack [1, 2, 3]
        r = B.pack [4, 5, 6]
    case ctSelectBytes l r 1 of
      Right out -> do
        out `shouldBe` l
        -- ByteStrings are immutable, so equal content is the observable
        -- contract; a copy is made by packZipWith.
        B.length out `shouldBe` 3
      Left e -> expectationFailure (show e)

  it "shows its errors" $ do
    show SelectLengthMismatch `shouldBe` "SelectLengthMismatch"
    show InvalidChoice `shouldBe` "InvalidChoice"

-- ---------------------------------------------------------------------------
-- Unsigned 64-bit equality
-- ---------------------------------------------------------------------------

u64Spec :: Spec
u64Spec = describe "ctEqU64" $ do
  it "is true for equal values including 0, max and the sign bit" $
    forM_ [0, 1, maxBound, 2 ^ (63 :: Int), 2 ^ (63 :: Int) - 1, 0x0123456789ABCDEF :: Word64] $ \v -> do
      ctEqU64 v v `shouldBe` True
      ctEqU64Bit v v `shouldBe` 1

  it "detects each of the 64 single-bit flips from several bases" $
    forM_ [0, maxBound, 0x5555555555555555, 0x0123456789ABCDEF, 2 ^ (63 :: Int) :: Word64] $ \base ->
      forM_ [0 .. 63 :: Int] $ \bit -> do
        let flipped = base `xor` (1 `shiftL` bit)
        ctEqU64 base flipped `shouldBe` False
        ctEqU64 flipped base `shouldBe` False
        ctEqU64Bit base flipped `shouldBe` 0

  it "distinguishes 0 from the sign bit and from the maximum" $ do
    ctEqU64 0 (2 ^ (63 :: Int)) `shouldBe` False
    ctEqU64 0 maxBound `shouldBe` False
    ctEqU64 (2 ^ (63 :: Int) - 1) maxBound `shouldBe` False
