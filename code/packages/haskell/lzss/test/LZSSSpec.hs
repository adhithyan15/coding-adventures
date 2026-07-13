module LZSSSpec (spec) where

import qualified Data.ByteString as BS
import Data.List (isInfixOf)
import Data.Word (Word8, Word16)
import LZSS
import Test.Hspec

spec :: Spec
spec = do
    describe "CMP02 vectors" $ do
        it "encodes empty input as no tokens" $
            encode BS.empty `shouldBe` []
        it "encodes a single byte as a literal" $
            encode (ascii "A") `shouldBe` [Literal 65]
        it "keeps non-repeating bytes literal" $
            encode (ascii "ABCDE")
                `shouldBe` map Literal [65, 66, 67, 68, 69]
        it "encodes the mixed AABCBBABC vector" $
            encode (ascii "AABCBBABC")
                `shouldBe`
                    [ Literal 65
                    , Literal 65
                    , Literal 66
                    , Literal 67
                    , Literal 66
                    , Literal 66
                    , Match 5 3
                    ]
        it "encodes ABABAB with an overlapping match" $
            encode (ascii "ABABAB")
                `shouldBe` [Literal 65, Literal 66, Match 2 4]
        it "encodes a repeated byte with a self-reference" $
            encode (ascii "AAAAAAA")
                `shouldBe` [Literal 65, Match 1 6]

    describe "round trips" $ do
        it "round-trips representative text and binary inputs" $ do
            mapM_
                (\input -> roundTrip input `shouldBe` Right input)
                [ BS.empty
                , ascii "A"
                , ascii "ABCDE"
                , ascii "AABCBBABC"
                , ascii "ABABAB"
                , ascii "hello world"
                , BS.pack [0, 0, 0, 255, 255]
                ]
        it "round-trips every byte value" $ do
            let input = BS.pack [0 .. 255]
            roundTrip input `shouldBe` Right input
        it "round-trips long repeated patterns" $ do
            let input = BS.concat (replicate 500 (ascii "ABCDEF"))
            roundTrip input `shouldBe` Right input
        it "round-trips a long single-byte run" $ do
            let input = BS.replicate 10000 66
            roundTrip input `shouldBe` Right input

    describe "parameters" $ do
        it "keeps offsets within the configured window" $ do
            tokens <- expectRight (encodeWith 4 255 3 (BS.concat (replicate 100 (ascii "ABCABC"))))
            mapM_ (offsetShouldBeAtMost 4) tokens
        it "keeps matches within the configured maximum" $ do
            tokens <- expectRight (encodeWith 4096 5 3 (BS.replicate 100 65))
            mapM_ (lengthShouldBeAtMost 5) tokens
        it "lets a high threshold force literal output" $ do
            tokens <- expectRight (encodeWith 4096 255 100 (ascii "ABABAB"))
            tokens `shouldSatisfy` all isLiteral
        it "rejects parameters that cannot fit the wire format" $ do
            encodeWith 0 255 3 (ascii "A") `leftShouldContain` "window size"
            encodeWith 65536 255 3 (ascii "A") `leftShouldContain` "uint16"
            encodeWith 4096 256 3 (ascii "A") `leftShouldContain` "uint8"
            encodeWith 4096 2 3 (ascii "A") `leftShouldContain` "exceeds"

    describe "decoding and validation" $ do
        it "copies overlapping matches byte by byte" $
            decode [Literal 65, Match 1 6]
                `shouldBe` Right (ascii "AAAAAAA")
        it "decodes non-overlapping matches" $
            decode [Literal 65, Literal 66, Literal 67, Match 3 3]
                `shouldBe` Right (ascii "ABCABC")
        it "uses the stored length as an authoritative truncation" $
            decodeWithLength 2 [Literal 65, Literal 66, Literal 67]
                `shouldBe` Right (ascii "AB")
        it "rejects output shorter than the stored length" $
            decodeWithLength 2 [Literal 65] `leftShouldContain` "shorter"
        it "rejects zero offsets and lengths" $ do
            decode [Literal 65, Match 0 3] `leftShouldContain` "positive offset"
            decode [Literal 65, Match 1 0] `leftShouldContain` "positive length"
        it "rejects offsets beyond the decoded prefix" $
            decode [Literal 65, Match 2 3] `leftShouldContain` "exceeds"

    describe "wire format" $ do
        it "writes the exact empty header" $
            compress BS.empty `shouldBe` Right (BS.replicate 8 0)
        it "writes the exact ABABAB flag block" $
            compress (ascii "ABABAB")
                `shouldBe`
                    Right
                        ( BS.pack
                            [ 0, 0, 0, 6
                            , 0, 0, 0, 1
                            , 4
                            , 65, 66
                            , 0, 2, 4
                            ]
                        )
        it "groups eight literals into one block" $
            compress (ascii "ABCDEFGH")
                `shouldBe`
                    Right
                        ( BS.pack
                            [ 0, 0, 0, 8
                            , 0, 0, 0, 1
                            , 0
                            , 65, 66, 67, 68, 69, 70, 71, 72
                            ]
                        )
        it "starts a second block for the ninth token" $ do
            payload <- expectRight (compress (ascii "ABCDEFGHI"))
            BS.take 8 payload
                `shouldBe` BS.pack [0, 0, 0, 9, 0, 0, 0, 2]
            BS.drop 8 payload
                `shouldBe` BS.pack [0, 65, 66, 67, 68, 69, 70, 71, 72, 0, 73]
        it "serialises and deserialises mixed tokens losslessly" $ do
            let tokens = [Literal 65, Literal 66, Match 2 4]
            payload <- expectRight (serialiseTokens 6 tokens)
            deserialiseTokens payload `shouldBe` Right (tokens, 6)
        it "rejects truncated headers and implausible block counts" $ do
            deserialiseTokens (BS.replicate 7 0) `leftShouldContain` "header"
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 0, 0, 2, 0])
                `leftShouldContain` "block count"
        it "rejects missing block payload and truncated match records" $ do
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 0, 0, 1])
                `leftShouldContain` "block count"
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1])
                `leftShouldContain` "match record"
        it "rejects set unused flag bits and trailing data" $ do
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 0, 0, 1, 2, 65])
                `leftShouldContain` "match record"
            deserialiseTokens (BS.pack [0, 0, 0, 0, 0, 0, 0, 0, 65])
                `leftShouldContain` "trailing"
        it "rejects invalid match fields from the wire" $ do
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 3])
                `leftShouldContain` "positive offset"
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 1, 0])
                `leftShouldContain` "positive length"

    describe "compression behaviour" $ do
        it "compresses repetitive input below its original size" $ do
            let input = BS.concat (replicate 1000 (ascii "ABC"))
            payload <- expectRight (compress input)
            BS.length payload `shouldSatisfy` (< BS.length input)
        it "compresses a long repeated byte below its original size" $ do
            let input = BS.replicate 10000 66
            payload <- expectRight (compress input)
            BS.length payload `shouldSatisfy` (< BS.length input)
        it "produces deterministic output" $
            compress (ascii "hello world test")
                `shouldBe` compress (ascii "hello world test")

roundTrip :: BS.ByteString -> Either String BS.ByteString
roundTrip input = compress input >>= decompress

ascii :: String -> BS.ByteString
ascii = BS.pack . map (fromIntegral . fromEnum)

isLiteral :: Token -> Bool
isLiteral Literal {} = True
isLiteral Match {} = False

offsetShouldBeAtMost :: Word16 -> Token -> Expectation
offsetShouldBeAtMost maximumOffset Match {matchOffset = offset} =
    offset `shouldSatisfy` (<= maximumOffset)
offsetShouldBeAtMost _ Literal {} = pure ()

lengthShouldBeAtMost :: Word8 -> Token -> Expectation
lengthShouldBeAtMost maximumLength Match {matchLength = lengthValue} =
    lengthValue `shouldSatisfy` (<= maximumLength)
lengthShouldBeAtMost _ Literal {} = pure ()

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
