module Lz78Spec (spec) where

import qualified Data.ByteString as BS
import Data.ByteString (ByteString)
import Lz78
import Test.Hspec

bytes :: String -> ByteString
bytes = BS.pack . map (fromIntegral . fromEnum)

roundTrip :: ByteString -> Either Lz78Error ByteString
roundTrip = decompress . compressDefault

spec :: Spec
spec = do
    describe "TrieCursor" $ do
        it "starts at the root and reports missing edges" $ do
            cursorAtRoot emptyCursor `shouldBe` True
            cursorDictId emptyCursor `shouldBe` 0
            stepCursor 65 emptyCursor `shouldBe` Nothing

        it "inserts, follows, and resets immutable edges" $ do
            let inserted = insertCursor 65 7 emptyCursor
                advanced = stepCursor 65 inserted
            fmap cursorAtRoot advanced `shouldBe` Just False
            fmap cursorDictId advanced `shouldBe` Just 7
            fmap (cursorAtRoot . resetCursor) advanced `shouldBe` Just True
            cursorAtRoot emptyCursor `shouldBe` True

    describe "encode and decode" $ do
        it "handles empty and single-byte inputs" $ do
            encode BS.empty defaultMaxDictionarySize `shouldBe` []
            encode (bytes "A") defaultMaxDictionarySize `shouldBe` [Token 0 65]
            decode [] (Just 0) `shouldBe` Right BS.empty
            decode [Token 0 65] (Just 1) `shouldBe` Right (bytes "A")

        it "matches the CMP01 AABCBBABC vector" $ do
            encode (bytes "AABCBBABC") defaultMaxDictionarySize
                `shouldBe` [ Token 0 65
                           , Token 1 66
                           , Token 0 67
                           , Token 0 66
                           , Token 4 65
                           , Token 4 67
                           ]

        it "emits the specified end-of-stream flush token" $ do
            let tokens = [Token 0 65, Token 0 66, Token 1 66, Token 3 0]
            encode (bytes "ABABAB") defaultMaxDictionarySize `shouldBe` tokens
            decode tokens (Just 6) `shouldBe` Right (bytes "ABABAB")
            decode tokens Nothing `shouldBe` Right (BS.snoc (bytes "ABABAB") 0)

        it "caps dictionary indices at the requested size" $ do
            let capped = encode (bytes "ABCABCABCABCABC") 3
                literalsOnly = encode (bytes "AAAA") 1
                clamped = encode (bytes "AAAA") 0
            all ((< 3) . dictIndex) capped `shouldBe` True
            map dictIndex literalsOnly `shouldBe` replicate 4 0
            clamped `shouldBe` literalsOnly

        it "round-trips text and arbitrary bytes" $ do
            let cases =
                    [ BS.empty
                    , bytes "A"
                    , bytes "ABCDE"
                    , bytes "AAAAAAA"
                    , bytes "ABABABAB"
                    , bytes "AABCBBABC"
                    , bytes "hello world"
                    , BS.pack [0, 0, 0, 255, 255]
                    , BS.pack [0 .. 255]
                    ]
            map roundTrip cases `shouldBe` map Right cases

        it "rejects invalid dictionary references and lengths" $ do
            decode [Token 2 65] (Just 1) `shouldBe` Left (InvalidDictionaryIndex 2 1)
            decode [Token 0 65] (Just (-1)) `shouldBe` Left (InvalidOriginalLength (-1))
            decode [Token 0 65] (Just 2) `shouldBe` Left (DecodedLengthMismatch 2 1)

    describe "wire format" $ do
        it "serializes fields in big-endian four-byte records" $ do
            serialiseTokens [Token 0x1234 0xab] 0x01020304
                `shouldBe` BS.pack [1, 2, 3, 4, 0, 0, 0, 1, 0x12, 0x34, 0xab, 0]

        it "deserializes complete token streams" $ do
            let tokens = [Token 0 65, Token 1 66]
                wire = serialiseTokens tokens 3
            deserialiseTokens wire `shouldBe` Right (tokens, 3)

        it "rejects short, truncated, and overlong streams" $ do
            deserialiseTokens (BS.pack [0, 1, 2]) `shouldBe` Left (HeaderTooShort 3)
            deserialiseTokens (BS.pack [0, 0, 0, 1, 0, 0, 0, 1])
                `shouldBe` Left (WireLengthMismatch 12 8)
            deserialiseTokens (BS.pack [0, 0, 0, 0, 0, 0, 0, 0, 1])
                `shouldBe` Left (WireLengthMismatch 8 9)

        it "rejects non-zero reserved token bytes" $ do
            let malformed = BS.pack [0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 65, 9]
            deserialiseTokens malformed `shouldBe` Left (NonZeroReservedByte 0 9)

        it "composes the one-shot API deterministically" $ do
            let input = bytes "ABCABCABCABC"
                compressed = compress input 32
            compressed `shouldBe` compress input 32
            decompress compressed `shouldBe` Right input
