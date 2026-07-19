module DeflateSpec (spec) where

import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BSC
import Data.Bits (shiftL)
import Data.Char (digitToInt)
import Data.Word (Word16)
import Deflate (compress, compressWith, decompress)
import Test.Hspec

spec :: Spec
spec = do
    describe "compress and decompress" $ do
        it "uses the canonical minimal stream for empty input" $ do
            let expected = BS.pack [0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1, 0]
            compress BS.empty `shouldBe` Right expected
            decompress expected `shouldBe` Right BS.empty

        it "round trips both extreme single-byte values" $ do
            roundTrip (BS.singleton 0)
            roundTrip (BS.singleton 255)

        it "omits the distance tree for a literal-only stream" $ do
            compressed <- expectRight (compress (BSC.pack "AAABBC"))
            readWord16 compressed 6 `shouldBe` 0
            decompress compressed `shouldBe` Right (BSC.pack "AAABBC")

        it "emits a distance tree for the CMP05 match example" $ do
            let input = BSC.pack "AABCBBABC"
            compressed <- expectRight (compress input)
            readWord16 compressed 6 `shouldSatisfy` (> 0)
            decompress compressed `shouldBe` Right input

        it "matches established cross-language CMP05 vectors" $ do
            compress (BSC.pack "AAABBC")
                `shouldBe` Right (hexBytes "0000000600040000004101004202004303010003a81d")
            compress (BSC.pack "AABCBBABC")
                `shouldBe` Right (hexBytes "0000000900050001004102004202010102004303010003000401e0340e")
            compress (BSC.pack "ABCABCABCABC")
                `shouldBe` Right (hexBytes "0000000c000500010043020100020107020041030042030002013b11")

        it "round trips match-heavy examples" $ do
            mapM_
                (roundTrip . BSC.pack)
                [ "AAAAAAA"
                , "ABABABABABAB"
                , "ABCABCABCABC"
                , "hello hello hello world"
                , "AABABC"
                ]

        it "round trips long repetitive text" $ do
            roundTrip (BSC.pack (concat (replicate 10 "the quick brown fox jumps over the lazy dog ")))

        it "round trips binary data" $ do
            roundTrip (BS.pack [fromIntegral (index `mod` 256) | index <- [0 .. 999 :: Int]])

        it "compresses repetitive data below half its original size" $ do
            let input = BSC.pack (concat (replicate 100 "ABCABC"))
            compressed <- expectRight (compress input)
            BS.length compressed `shouldSatisfy` (< BS.length input `div` 2)

    describe "parameter validation" $ do
        it "rejects windows beyond the CMP05 distance table" $
            compressWith 4097 255 3 (BSC.pack "payload")
                `shouldBe` Left "CMP05 window size must not exceed 4096"

        it "rejects match thresholds below the CMP05 length table" $
            compressWith 4096 255 2 (BSC.pack "payload")
                `shouldBe` Left "CMP05 minimum match length must be at least 3"

        it "preserves LZSS parameter validation" $
            compressWith 4096 2 3 (BSC.pack "payload")
                `shouldBe` Left "minimum match length exceeds maximum match length"

    describe "malformed streams" $ do
        it "rejects a truncated header" $
            decompress (BS.replicate 7 0) `shouldBe` Left "truncated CMP05 header"

        it "rejects a truncated code-length table" $
            decompress (BS.pack [0, 0, 0, 0, 0, 1, 0, 0, 1, 0])
                `shouldBe` Left "truncated CMP05 literal/length code-length table"

        it "requires the end-of-block symbol" $
            decompress (BS.pack [0, 0, 0, 1, 0, 1, 0, 0, 0, 65, 1, 0])
                `shouldBe` Left "literal/length table is missing the end-of-block symbol"

        it "rejects unknown literal-length symbols" $
            decompress (BS.pack [0, 0, 0, 1, 0, 2, 0, 0, 1, 0, 1, 1, 29, 1, 0])
                `shouldBe` Left "unknown literal/length symbol 285"

        it "rejects alphabet counts beyond CMP05 bounds" $ do
            decompress (BS.pack [0, 0, 0, 0, 1, 30, 0, 0])
                `shouldBe` Left "literal/length table exceeds the CMP05 alphabet"
            decompress (BS.pack [0, 0, 0, 0, 0, 1, 0, 25])
                `shouldBe` Left "distance table exceeds the CMP05 alphabet"

        it "rejects unknown distance symbols" $
            decompress (BS.pack [0, 0, 0, 0, 0, 1, 0, 1, 1, 0, 1, 0, 24, 1, 0])
                `shouldBe` Left "unknown distance symbol 24"

        it "rejects zero, duplicate, and unsorted code tables" $ do
            decompress (BS.pack [0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0])
                `shouldBe` Left "literal/length code length is zero for symbol 256"
            decompress (BS.pack [0, 0, 0, 0, 0, 2, 0, 0, 1, 0, 1, 1, 0, 1, 0])
                `shouldBe` Left "duplicate symbol in CMP05 literal/length table"
            decompress (BS.pack [0, 0, 0, 0, 0, 2, 0, 0, 1, 0, 2, 0, 65, 1, 0])
                `shouldBe` Left "CMP05 literal/length table is not sorted by length and symbol"

        it "requires length one for single-symbol trees" $
            decompress (BS.pack [0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 2, 0])
                `shouldBe` Left "single-symbol literal/length table must use code length 1"

        it "rejects oversubscribed canonical lengths" $
            decompress (BS.pack [0, 0, 0, 0, 0, 3, 0, 0, 0, 65, 1, 0, 66, 1, 1, 0, 1, 0])
                `shouldBe` Left "oversubscribed CMP05 literal/length code lengths"

        it "requires a distance tree for match symbols" $
            decompress (BS.pack [0, 0, 0, 3, 0, 2, 0, 0, 1, 0, 1, 1, 1, 1, 1])
                `shouldBe` Left "compressed stream references a missing distance tree"

        it "rejects backreferences before the decoded prefix" $
            decompress (BS.pack [0, 0, 0, 3, 0, 2, 0, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 1])
                `shouldBe` Left "CMP05 distance extends before the output buffer"

        it "checks declared output length at literals and end markers" $ do
            decompress (BS.pack [0, 0, 0, 0, 0, 2, 0, 0, 0, 65, 1, 1, 0, 1, 0])
                `shouldBe` Left "decoded data exceeds the CMP05 header length"
            decompress (BS.pack [0, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0])
                `shouldBe` Left "decoded length 0 does not match CMP05 header length 1"

        it "rejects invalid canonical prefixes" $
            decompress (BS.pack [0, 0, 0, 0, 0, 1, 0, 0, 1, 0, 1, 1])
                `shouldBe` Left "invalid prefix in CMP05 literal/length bit stream"

        it "rejects truncated compressed bits" $ do
            compressed <- expectRight (compress (BSC.pack "ABCABCABCABCABCABC"))
            decompress (BS.init compressed) `shouldSatisfy` isLeft

roundTrip :: BS.ByteString -> Expectation
roundTrip input = (compress input >>= decompress) `shouldBe` Right input

expectRight :: (Show failure) => Either failure value -> IO value
expectRight result =
    case result of
        Left failure -> expectationFailure (show failure) >> fail "unreachable"
        Right value -> pure value

isLeft :: Either value result -> Bool
isLeft (Left _) = True
isLeft (Right _) = False

readWord16 :: BS.ByteString -> Int -> Word16
readWord16 input offset =
    (fromIntegral (BS.index input offset) `shiftL` 8)
        + fromIntegral (BS.index input (offset + 1))

hexBytes :: String -> BS.ByteString
hexBytes [] = BS.empty
hexBytes (high : low : rest) =
    BS.cons (fromIntegral (digitToInt high * 16 + digitToInt low)) (hexBytes rest)
hexBytes _ = error "hex fixture must contain complete bytes"
