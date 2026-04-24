module LzssSpec (spec) where

import Test.Hspec
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BC

import Lzss

-- ─── Helpers ──────────────────────────────────────────────────────────────────

-- | Convenience: compress then decompress.
rt :: ByteString -> ByteString
rt = decompress . compress

-- | Encode with default parameters.
enc :: ByteString -> [Token]
enc = encode defaultWindowSize defaultMaxMatch defaultMinMatch

-- ─── Test suite ───────────────────────────────────────────────────────────────

spec :: Spec
spec = do

    -- ── 1. Round-trip: compress/decompress ────────────────────────────────────
    describe "round-trip (compress/decompress)" $ do

        it "empty input round-trips" $
            rt BS.empty `shouldBe` BS.empty

        it "single byte round-trips" $
            rt (BS.singleton 0x41) `shouldBe` BS.singleton 0x41

        it "no-repetition string round-trips" $
            rt (BC.pack "ABCDE") `shouldBe` BC.pack "ABCDE"

        it "repetitive input round-trips" $
            rt (BC.pack "ABABABABAB") `shouldBe` BC.pack "ABABABABAB"

        it "all-identical bytes round-trip" $
            rt (BS.replicate 100 0x42) `shouldBe` BS.replicate 100 0x42

        it "hello world round-trips" $
            rt (BC.pack "hello world") `shouldBe` BC.pack "hello world"

        it "binary data with null bytes round-trips" $
            rt (BS.pack [0, 0, 0, 255, 255]) `shouldBe` BS.pack [0, 0, 0, 255, 255]

        it "long repeated pattern round-trips" $ do
            let dat = BS.concat (replicate 500 (BC.pack "ABC"))
            rt dat `shouldBe` dat

        it "longer string round-trips" $ do
            let dat = BC.pack "the quick brown fox jumps over the lazy dog"
            rt dat `shouldBe` dat

    -- ── 2. All 256 byte values round-trip ─────────────────────────────────────
    describe "all 256 byte values" $
        it "every possible byte value survives a round-trip" $ do
            let dat = BS.pack [0 .. 255]
            rt dat `shouldBe` dat

    -- ── 3. Repetitive input compresses ────────────────────────────────────────
    describe "compression effectiveness" $ do

        it "repetitive input is smaller after compression" $ do
            let dat = BS.concat (replicate 1000 (BC.pack "ABC"))
            BS.length (compress dat) < BS.length dat `shouldBe` True

        it "all-same-byte input compresses" $ do
            let dat = BS.replicate 10000 0x42
            BS.length (compress dat) < BS.length dat `shouldBe` True

    -- ── 4. encode: unique data produces only Literals ─────────────────────────
    describe "encode -- unique data" $ do

        it "single byte encodes to a single Literal" $
            enc (BS.singleton 0x41) `shouldBe` [Literal 0x41]

        it "no-repetition input is all Literals" $ do
            let toks = enc (BC.pack "ABCDE")
            all isLiteral toks `shouldBe` True
            length toks `shouldBe` 5

        it "empty input encodes to empty list" $
            enc BS.empty `shouldBe` []

    -- ── 5. encode: repeated data produces Match tokens ────────────────────────
    describe "encode -- repeated data" $ do

        it "ABABAB encodes to two Literals + one Match" $ do
            let toks = enc (BC.pack "ABABAB")
            toks `shouldBe`
                [ Literal 0x41
                , Literal 0x42
                , Match 2 4
                ]

        it "AAAAAAA encodes to one Literal + one Match" $ do
            let toks = enc (BC.pack "AAAAAAA")
            toks `shouldBe`
                [ Literal 0x41
                , Match 1 6
                ]

        it "AABCBBABC encodes to 6 Literals + 1 Match(offset=5,length=3)" $ do
            let toks = enc (BC.pack "AABCBBABC")
            length toks `shouldBe` 7
            last toks `shouldBe` Match 5 3

    -- ── 6. decode is the left-inverse of encode ───────────────────────────────
    describe "decode . encode == id" $ do

        it "works for empty input" $
            decode (enc BS.empty) `shouldBe` BS.empty

        it "works for single byte" $
            decode (enc (BS.singleton 0x41)) `shouldBe` BS.singleton 0x41

        it "works for all-literals" $
            decode (enc (BC.pack "ABCDE")) `shouldBe` BC.pack "ABCDE"

        it "works for ABABAB" $
            decode (enc (BC.pack "ABABAB")) `shouldBe` BC.pack "ABABAB"

        it "works for AAAAAAA" $
            decode (enc (BC.pack "AAAAAAA")) `shouldBe` BC.pack "AAAAAAA"

        it "works for AABCBBABC" $
            decode (enc (BC.pack "AABCBBABC")) `shouldBe` BC.pack "AABCBBABC"

    -- ── 7. Known decode vectors (from spec) ───────────────────────────────────
    describe "decode -- known vectors" $ do

        it "single Literal(A) decodes to A" $
            decode [Literal 0x41] `shouldBe` BS.singleton 0x41

        it "overlapping match: [Lit A, Match 1 6] -> AAAAAAA" $
            decode [Literal 0x41, Match 1 6] `shouldBe` BC.pack "AAAAAAA"

        it "ABABAB from two Literals + Match(2,4)" $
            decode [Literal 0x41, Literal 0x42, Match 2 4]
                `shouldBe` BC.pack "ABABAB"

    -- ── 8. Wire format properties ─────────────────────────────────────────────
    describe "wire format" $ do

        it "compress stores original length in first 4 bytes" $ do
            let compressed = compress (BC.pack "hello")
                storedLen  = fromIntegral (BS.index compressed 0) * 256 * 256 * 256
                           + fromIntegral (BS.index compressed 1) * 256 * 256
                           + fromIntegral (BS.index compressed 2) * 256
                           + fromIntegral (BS.index compressed 3) :: Int
            storedLen `shouldBe` 5

        it "compress is deterministic" $
            compress (BC.pack "hello world test")
                `shouldBe` compress (BC.pack "hello world test")

        it "empty input compresses to 8-byte header with original_length=0" $ do
            let compressed = compress BS.empty
            BS.length compressed `shouldBe` 8
            BS.index compressed 0 `shouldBe` 0
            BS.index compressed 1 `shouldBe` 0
            BS.index compressed 2 `shouldBe` 0
            BS.index compressed 3 `shouldBe` 0

        it "crafted large block_count does not panic" $ do
            let bad = BS.pack (replicate 4 0x00 ++ [0x40, 0x00, 0x00, 0x00] ++ replicate 8 0x00)
            BS.length (decompress bad) `shouldBe` 0

    -- ── 9. Match token properties ─────────────────────────────────────────────
    describe "match token properties" $ do

        it "all Match offsets are positive" $ do
            let toks = enc (BC.pack "ABABABABABAB")
            all positiveOffset toks `shouldBe` True

        it "all Match lengths are >= minMatch" $ do
            let toks = enc (BC.pack "ABABABABABAB")
            all (matchLengthGe defaultMinMatch) toks `shouldBe` True

        it "large minMatch forces all Literals" $ do
            let toks = encode defaultWindowSize defaultMaxMatch 100 (BC.pack "ABABAB")
            all isLiteral toks `shouldBe` True

        it "match lengths are <= maxMatch" $ do
            let toks = encode defaultWindowSize 5 defaultMinMatch (BS.replicate 100 0x41)
            all (matchLengthLe 5) toks `shouldBe` True

-- ─── Predicate helpers ────────────────────────────────────────────────────────

isLiteral :: Token -> Bool
isLiteral (Literal _) = True
isLiteral _           = False

positiveOffset :: Token -> Bool
positiveOffset (Match off _) = off >= 1
positiveOffset _             = True

matchLengthGe :: Int -> Token -> Bool
matchLengthGe minLen (Match _ ml) = ml >= minLen
matchLengthGe _      _            = True

matchLengthLe :: Int -> Token -> Bool
matchLengthLe maxLen (Match _ ml) = ml <= maxLen
matchLengthLe _      _            = True
