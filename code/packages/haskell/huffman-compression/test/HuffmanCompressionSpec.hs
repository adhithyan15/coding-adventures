module HuffmanCompressionSpec (spec) where

import qualified Data.ByteString as BS
import Data.Either (isLeft)
import Data.List (isInfixOf, sort)
import Data.Word (Word8, Word32)
import HuffmanCompression
import Test.Hspec

spec :: Spec
spec = do
    describe "round trips" $ do
        it "round-trips empty input" $
            roundTrip BS.empty `shouldBe` Right BS.empty
        it "round-trips the AAABBC reference vector" $
            roundTrip (ascii "AAABBC") `shouldBe` Right (ascii "AAABBC")
        it "round-trips hello world and binary bytes" $ do
            roundTrip (ascii "hello world") `shouldBe` Right (ascii "hello world")
            let binary = BS.pack [0, 1, 2, 3, 255, 254, 128, 64, 32]
            roundTrip binary `shouldBe` Right binary
        it "round-trips all 256 byte values" $ do
            let allBytes = BS.pack [0 .. 255]
            roundTrip allBytes `shouldBe` Right allBytes
        it "round-trips all byte values repeated" $ do
            let allBytes = BS.concat (replicate 10 (BS.pack [0 .. 255]))
            roundTrip allBytes `shouldBe` Right allBytes
        it "round-trips one repeated symbol" $ do
            let repeated = BS.replicate 100 65
            roundTrip repeated `shouldBe` Right repeated
        it "round-trips two equal-frequency symbols" $ do
            let alternating = BS.concat (replicate 100 (ascii "AB"))
            roundTrip alternating `shouldBe` Right alternating
        it "round-trips a long repeated pattern" $ do
            let patternBytes = BS.concat (replicate 500 (ascii "the quick brown fox "))
            roundTrip patternBytes `shouldBe` Right patternBytes

    describe "wire format" $ do
        it "writes the exact empty header" $
            compress BS.empty `shouldBe` Right (BS.replicate 8 0)
        it "writes the exact AAABBC bytes" $
            compress (ascii "AAABBC") `shouldBe`
                Right (BS.pack [0, 0, 0, 6, 0, 0, 0, 3, 65, 1, 66, 2, 67, 2, 0xA8, 0x01])
        it "writes the original length and symbol count as big-endian uint32 values" $ do
            compressed <- expectRight (compress (ascii "hello"))
            BS.take 4 compressed `shouldBe` BS.pack [0, 0, 0, 5]
            BS.take 4 (BS.drop 4 compressed) `shouldBe` BS.pack [0, 0, 0, 4]
        it "sorts table entries by code length and symbol" $ do
            compressed <- expectRight (compress (ascii "AAABBC"))
            let pairs = [(BS.index compressed offset, BS.index compressed (offset + 1)) | offset <- [8, 10, 12]]
            map (\(symbol, codeLength) -> (codeLength, symbol)) pairs
                `shouldBe` sort (map (\(symbol, codeLength) -> (codeLength, symbol)) pairs)
        it "uses one bit per occurrence for a single symbol" $ do
            compressed <- expectRight (compress (BS.replicate 8 65))
            BS.length compressed `shouldBe` 11
            BS.drop 8 compressed `shouldBe` BS.pack [65, 1, 0]
        it "packs the bit stream LSB-first" $ do
            compressed <- expectRight (compress (ascii "AAABBC"))
            BS.drop 14 compressed `shouldBe` BS.pack [0xA8, 0x01]

    describe "compression effectiveness and stability" $ do
        it "shrinks a highly repetitive input" $ do
            let input = BS.replicate 1000 88
            compressed <- expectRight (compress input)
            BS.length compressed `shouldSatisfy` (< BS.length input)
        it "expands a short uniform alphabet because of metadata" $ do
            let input = BS.pack [0 .. 255]
            compressed <- expectRight (compress input)
            BS.length compressed `shouldSatisfy` (> BS.length input)
        it "produces deterministic output" $ do
            let input = ascii "the quick brown fox jumps over the lazy dog"
            compress input `shouldBe` compress input

    describe "malformed input" $ do
        it "rejects truncated headers" $ do
            decompress BS.empty `shouldSatisfy` isLeft
            decompress (BS.replicate 7 0) `shouldSatisfy` isLeft
        it "rejects a zero symbol count for non-empty output" $
            decompress (wire 1 0 []) `leftShouldContain` "at least one symbol"
        it "rejects a non-zero symbol count for empty output" $
            decompress (wire 0 1 [65, 1]) `leftShouldContain` "zero symbol count"
        it "rejects symbol counts larger than the byte alphabet" $
            decompress (wire 1 257 []) `leftShouldContain` "256-byte alphabet"
        it "rejects truncated code-length tables" $
            decompress (wire 1 2 [65, 1]) `leftShouldContain` "truncated"
        it "rejects zero and overlong code lengths" $ do
            decompress (wire 1 1 [65, 0, 0]) `leftShouldContain` "length is zero"
            decompress (wire 1 1 [65, 17, 0]) `leftShouldContain` "exceeds 16"
        it "rejects duplicate symbols" $
            decompress (wire 1 2 [65, 1, 65, 2, 0]) `leftShouldContain` "duplicate symbol"
        it "rejects unsorted code-length entries" $
            decompress (wire 1 2 [66, 2, 65, 1, 0]) `leftShouldContain` "not sorted"
        it "rejects oversubscribed canonical lengths" $
            decompress (wire 1 3 [65, 1, 66, 1, 67, 1, 0]) `leftShouldContain` "oversubscribed"
        it "rejects exhausted bit streams" $
            decompress (wire 2 1 [65, 1]) `leftShouldContain` "exhausted"
        it "rejects invalid bit prefixes" $
            decompress (wire 1 1 [65, 1, 1]) `leftShouldContain` "invalid prefix"

roundTrip :: BS.ByteString -> Either String BS.ByteString
roundTrip input = compress input >>= decompress

ascii :: String -> BS.ByteString
ascii = BS.pack . map (fromIntegral . fromEnum)

expectRight :: Either String value -> IO value
expectRight result =
    case result of
        Left message -> expectationFailure message >> fail message
        Right value -> pure value

leftShouldContain :: Either String value -> String -> Expectation
leftShouldContain result expected =
    case result of
        Left message -> message `shouldSatisfy` isInfixOf expected
        Right _ -> expectationFailure ("expected Left containing " ++ show expected)

wire :: Word32 -> Word32 -> [Word8] -> BS.ByteString
wire originalLength symbolCount rest =
    BS.pack (word32 originalLength ++ word32 symbolCount ++ rest)

word32 :: Word32 -> [Word8]
word32 value =
    [ fromIntegral (value `div` 16777216)
    , fromIntegral (value `div` 65536)
    , fromIntegral (value `div` 256)
    , fromIntegral value
    ]
