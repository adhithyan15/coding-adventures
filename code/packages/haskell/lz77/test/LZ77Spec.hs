module LZ77Spec (spec) where

import qualified Data.ByteString as BS
import Data.Either (isLeft)
import Data.List (isInfixOf)
import LZ77
import Test.Hspec

spec :: Spec
spec = do
    describe "spec vectors" $ do
        it "encodes empty input as no tokens" $ do
            encode BS.empty `shouldBe` []
            decode [] `shouldBe` Right BS.empty
        it "encodes ABCDE as literals" $
            encode (ascii "ABCDE")
                `shouldBe` map (Token 0 0) [65, 66, 67, 68, 69]
        it "encodes seven identical bytes with an overlapping match" $
            encode (ascii "AAAAAAA")
                `shouldBe` [Token 0 0 65, Token 1 5 65]
        it "encodes ABABABAB with the reference backreference" $
            encode (ascii "ABABABAB")
                `shouldBe` [Token 0 0 65, Token 0 0 66, Token 2 5 66]
        it "keeps AABCBBABC literal-only at the default threshold" $
            encode (ascii "AABCBBABC")
                `shouldBe` map (Token 0 0) [65, 65, 66, 67, 66, 66, 65, 66, 67]
        it "round-trips AABCBBABC with a lower match threshold" $ do
            tokens <- expectRight (encodeWith 4096 255 2 (ascii "AABCBBABC"))
            decode tokens `shouldBe` Right (ascii "AABCBBABC")

    describe "round trips" $ do
        it "round-trips representative text and binary data" $ do
            mapM_
                (\input -> decode (encode input) `shouldBe` Right input)
                [ BS.empty
                , ascii "A"
                , ascii "hello world"
                , ascii "the quick brown fox"
                , ascii "ababababab"
                , ascii "aaaaaaaaaa"
                , BS.pack [0, 0, 0]
                , BS.pack [255, 255, 255]
                ]
        it "round-trips all 256 byte values" $ do
            let input = BS.pack [0 .. 255]
            decode (encode input) `shouldBe` Right input
        it "round-trips the one-shot wire format" $ do
            mapM_
                (\input -> roundTrip input `shouldBe` Right input)
                [BS.empty, ascii "A", ascii "ABCDE", ascii "AAAAAAA", ascii "ABABABAB"]
        it "round-trips long repetitive input" $ do
            let input = BS.concat (replicate 100 (ascii "Hello, World! ")) <> BS.replicate 500 88
            decode (encode input) `shouldBe` Right input

    describe "parameters" $ do
        it "never emits offsets beyond the configured window" $ do
            let input = BS.singleton 88 <> BS.replicate 5000 89 <> BS.singleton 88
            tokens <- expectRight (encodeWith 100 255 3 input)
            map tokenOffset tokens `shouldSatisfy` all (<= 100)
        it "never emits lengths beyond the configured maximum" $ do
            tokens <- expectRight (encodeWith 4096 50 3 (BS.replicate 1000 65))
            map tokenLength tokens `shouldSatisfy` all (<= 50)
        it "rejects parameters that cannot fit the wire format" $ do
            encodeWith 0 255 3 (ascii "A") `leftShouldContain` "window size"
            encodeWith 65536 255 3 (ascii "A") `leftShouldContain` "uint16"
            encodeWith 4096 256 3 (ascii "A") `leftShouldContain` "uint8"
            encodeWith 4096 2 3 (ascii "A") `leftShouldContain` "exceeds"

    describe "overlapping decoding and validation" $ do
        it "copies overlapping matches byte by byte" $
            decode [Token 0 0 65, Token 0 0 66, Token 2 5 90]
                `shouldBe` Right (ascii "ABABABAZ")
        it "uses and returns an initial search buffer" $
            decodeWithInitialBuffer (ascii "AB") [Token 2 3 90]
                `shouldBe` Right (ascii "ABABAZ")
        it "rejects zero-offset backreferences" $
            decode [Token 0 3 90] `leftShouldContain` "positive offset"
        it "rejects offsets beyond the decoded prefix" $
            decodeWithInitialBuffer (ascii "A") [Token 2 1 90]
                `leftShouldContain` "exceeds decoded prefix"
        it "rejects literal tokens with non-zero offsets" $
            decode [Token 1 0 90] `leftShouldContain` "literal"

    describe "wire format" $ do
        it "writes the exact empty token-count header" $
            compress BS.empty `shouldBe` Right (BS.replicate 4 0)
        it "writes token count and fixed-width fields in big-endian order" $
            serialiseTokens [Token 0 0 65, Token 0x0201 5 66]
                `shouldBe` Right (BS.pack [0, 0, 0, 2, 0, 0, 0, 65, 2, 1, 5, 66])
        it "writes the exact ABABABAB reference payload" $
            compress (ascii "ABABABAB")
                `shouldBe`
                    Right
                        ( BS.pack
                            [ 0, 0, 0, 3
                            , 0, 0, 0, 65
                            , 0, 0, 0, 66
                            , 0, 2, 5, 66
                            ]
                        )
        it "serialises and deserialises tokens losslessly" $ do
            let tokens = [Token 0 0 65, Token 1 3 66, Token 2 5 67]
            serialised <- expectRight (serialiseTokens tokens)
            deserialiseTokens serialised `shouldBe` Right tokens
        it "treats an empty byte string as an empty token stream" $
            deserialiseTokens BS.empty `shouldBe` Right []
        it "rejects partial headers and truncated token streams" $ do
            deserialiseTokens (BS.pack [0, 0, 0]) `leftShouldContain` "header"
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 2, 3])
                `leftShouldContain` "truncated"
        it "rejects malformed token fields during deserialisation" $ do
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 0, 3, 90])
                `leftShouldContain` "positive offset"
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 1, 0, 90])
                `leftShouldContain` "literal"

    describe "compression behaviour" $ do
        it "compresses a long repeated byte into fewer than fifty tokens" $ do
            let tokens = encode (BS.replicate 10000 65)
            length tokens `shouldSatisfy` (< 50)
            decode tokens `shouldBe` Right (BS.replicate 10000 65)
        it "keeps incompressible input within fixed-width overhead" $ do
            let input = BS.pack [0 .. 255]
            compressed <- expectRight (compress input)
            BS.length compressed `shouldSatisfy` (<= 4 * BS.length input + 10)
        it "compresses repetitive input to fewer bytes" $ do
            let input = BS.concat (replicate 100 (ascii "ABC"))
            compressed <- expectRight (compress input)
            BS.length compressed `shouldSatisfy` (< BS.length input)
        it "produces deterministic output" $ do
            let input = ascii "hello world test"
            compress input `shouldBe` compress input

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
