module LZWSpec (spec) where

import qualified Data.ByteString as BS
import Data.Bits (shiftR)
import Data.List (isInfixOf)
import Data.Word (Word32)
import LZW
import Test.Hspec

spec :: Spec
spec = do
    describe "CMP03 code vectors" $ do
        it "encodes empty input" $
            encode BS.empty `shouldBe` [clearCode, stopCode]
        it "encodes a single byte" $
            encode (ascii "A") `shouldBe` [clearCode, 65, stopCode]
        it "encodes two distinct bytes" $
            encode (ascii "AB") `shouldBe` [clearCode, 65, 66, stopCode]
        it "encodes the repeated-pair vector" $
            encode (ascii "ABABAB")
                `shouldBe` [clearCode, 65, 66, 258, 258, stopCode]
        it "encodes the tricky-token vector" $
            encode (ascii "AAAAAAA")
                `shouldBe` [clearCode, 65, 258, 259, 65, stopCode]
        it "decodes every reference vector" $ do
            decode [clearCode, stopCode] `shouldBe` Right BS.empty
            decode [clearCode, 65, stopCode] `shouldBe` Right (ascii "A")
            decode [clearCode, 65, 66, stopCode] `shouldBe` Right (ascii "AB")
            decode [clearCode, 65, 66, 258, 258, stopCode]
                `shouldBe` Right (ascii "ABABAB")
            decode [clearCode, 65, 258, 259, 65, stopCode]
                `shouldBe` Right (ascii "AAAAAAA")

    describe "round trips" $ do
        it "round-trips representative text and binary inputs" $ do
            mapM_
                (\input -> roundTrip input `shouldBe` Right input)
                [ BS.empty
                , ascii "A"
                , ascii "AB"
                , ascii "ABABAB"
                , ascii "AAAAAAA"
                , ascii "hello world"
                , BS.pack [0, 0, 0, 255, 255]
                ]
        it "round-trips every byte value" $
            roundTrip (BS.pack [0 .. 255]) `shouldBe` Right (BS.pack [0 .. 255])
        it "round-trips beyond the 9-bit width" $ do
            let input = BS.pack (take 4096 (cycle [0 .. 255]))
            roundTrip input `shouldBe` Right input
        it "round-trips long repeated patterns" $ do
            let input = BS.concat (replicate 10000 (ascii "ABCDEFGHIJ"))
            roundTrip input `shouldBe` Right input
        it "round-trips a long single-byte run" $ do
            let input = BS.replicate 10000 66
            roundTrip input `shouldBe` Right input

    describe "wire format" $ do
        it "writes the exact empty payload" $
            compress BS.empty
                `shouldBe` Right (BS.pack [0, 0, 0, 0, 0, 3, 2])
        it "matches the cross-language bytes for the reference vectors" $ do
            compress (ascii "A")
                `shouldBe` Right (BS.pack [0, 0, 0, 1, 0, 131, 4, 4])
            compress (ascii "AB")
                `shouldBe` Right (BS.pack [0, 0, 0, 2, 0, 131, 8, 9, 8])
            compress (ascii "ABABAB")
                `shouldBe` Right (BS.pack [0, 0, 0, 6, 0, 131, 8, 17, 40, 48, 32])
            compress (ascii "AAAAAAA")
                `shouldBe` Right (BS.pack [0, 0, 0, 7, 0, 131, 8, 28, 24, 36, 32])
        it "stores the original length as big-endian uint32" $ do
            payload <- expectRight (compress (ascii "hello"))
            BS.take 4 payload `shouldBe` BS.pack [0, 0, 0, 5]
        it "packs one 9-bit CLEAR code LSB-first" $
            packCodes 0 [clearCode, stopCode]
                `shouldBe` Right (BS.pack [0, 0, 0, 0, 0, 3, 2])
        it "round-trips logical codes through variable-width packing" $ do
            let input = BS.pack (take 4096 (cycle [0 .. 255]))
                codes = encode input
            payload <- expectRight (packCodes (BS.length input) codes)
            unpackCodes payload `shouldBe` Right (codes, BS.length input)
        it "accepts a mid-stream CLEAR reset" $ do
            let codes = [clearCode, 65, clearCode, 66, stopCode]
            payload <- expectRight (packCodes 2 codes)
            decompress payload `shouldBe` Right (ascii "AB")
        it "uses the stored length as authoritative truncation" $ do
            payload <- expectRight (packCodes 1 [clearCode, 65, 66, stopCode])
            decompress payload `shouldBe` Right (ascii "A")
        it "produces deterministic output" $
            compress (ascii "hello world test")
                `shouldBe` compress (ascii "hello world test")
        it "emits CLEAR and round-trips when the 16-bit dictionary fills" $ do
            let input = pseudoRandomBytes 100000
                codes = encode input
            length (filter (== clearCode) codes) `shouldSatisfy` (> 1)
            roundTrip input `shouldBe` Right input

    describe "validation" $ do
        it "rejects a truncated header and empty payload" $ do
            decompress (BS.replicate 3 0) `leftShouldContain` "header"
            decompress (BS.replicate 4 0) `leftShouldContain` "truncated"
        it "requires CLEAR as the opening code" $ do
            packCodes 1 [65, stopCode] `leftShouldContain` "CLEAR"
            decompress (BS.pack [0, 0, 0, 1, 65, 0]) `leftShouldContain` "CLEAR"
        it "requires a STOP code" $ do
            packCodes 1 [clearCode, 65] `leftShouldContain` "STOP"
            decompress (BS.pack [0, 0, 0, 0, 0, 1]) `leftShouldContain` "truncated"
        it "rejects truly invalid and initial tricky codes" $ do
            decode [clearCode, 400, stopCode] `leftShouldContain` "invalid"
            decode [clearCode, 258, stopCode] `leftShouldContain` "previous"
        it "rejects non-zero data after STOP" $ do
            payload <- expectRight (compress BS.empty)
            decompress (payload <> BS.singleton 1) `leftShouldContain` "non-zero"
        it "rejects extra zero bytes after STOP" $ do
            payload <- expectRight (compress BS.empty)
            decompress (payload <> BS.singleton 0) `leftShouldContain` "trailing"
        it "rejects decoded output shorter than the stored length" $ do
            payload <- expectRight (packCodes 2 [clearCode, 65, stopCode])
            decompress payload `leftShouldContain` "shorter"
        it "rejects code data after STOP when packing" $
            packCodes 0 [clearCode, stopCode, 65] `leftShouldContain` "after STOP"

    describe "compression behaviour" $ do
        it "compresses repetitive input below its original size" $ do
            let input = BS.concat (replicate 1000 (ascii "ABC"))
            payload <- expectRight (compress input)
            BS.length payload `shouldSatisfy` (< BS.length input)
        it "compresses a long repeated byte below its original size" $ do
            let input = BS.replicate 10000 66
            payload <- expectRight (compress input)
            BS.length payload `shouldSatisfy` (< BS.length input)

roundTrip :: BS.ByteString -> Either String BS.ByteString
roundTrip input = compress input >>= decompress

ascii :: String -> BS.ByteString
ascii = BS.pack . map (fromIntegral . fromEnum)

pseudoRandomBytes :: Int -> BS.ByteString
pseudoRandomBytes count =
    BS.pack
        ( map (fromIntegral . (`shiftR` 24))
            (take count (drop 1 (iterate nextState (1 :: Word32))))
        )
  where
    nextState state = 1664525 * state + 1013904223

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
