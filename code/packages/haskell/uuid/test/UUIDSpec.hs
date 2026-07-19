module UUIDSpec (spec) where

import Control.Concurrent (threadDelay)
import Data.Bits ((.&.), shiftL)
import Data.Either (isLeft)
import Data.List (nub)
import Data.Time.Clock.POSIX (getPOSIXTime)
import Data.Word (Word8)
import Test.Hspec
import UUID

sample :: UUID
sample = expectRight (parse "550e8400-e29b-41d4-a716-446655440000")

spec :: Spec
spec = do
    describe "construction and conversion" $ do
        it "accepts exactly sixteen bytes" $ do
            uuidBytes (expectRight (uuidFromBytes [0 .. 15])) `shouldBe` [0 .. 15]
            uuidFromBytes (replicate 15 0) `shouldSatisfy` isLeft
            uuidFromBytes (replicate 17 0) `shouldSatisfy` isLeft
        it "round-trips unsigned 128-bit integers" $ do
            uuidToInteger (expectRight (uuidFromInteger 0)) `shouldBe` 0
            uuidToInteger (expectRight (uuidFromInteger (2 ^ (128 :: Int) - 1)))
                `shouldBe` 2 ^ (128 :: Int) - 1
            uuidToInteger (expectRight (uuidFromInteger 0x550e8400e29b41d4a716446655440000))
                `shouldBe` 0x550e8400e29b41d4a716446655440000
        it "rejects integers outside the unsigned 128-bit range" $ do
            uuidFromInteger (-1) `shouldSatisfy` isLeft
            uuidFromInteger (2 ^ (128 :: Int)) `shouldSatisfy` isLeft

    describe "parsing and rendering" $ do
        it "renders lowercase canonical text" $ do
            render sample `shouldBe` "550e8400-e29b-41d4-a716-446655440000"
            show sample `shouldContain` render sample
        it "accepts uppercase, compact, braced, URN, whitespace, and mixed separators" $
            mapM_
                (\value -> parse value `shouldBe` Right sample)
                [ "550E8400-E29B-41D4-A716-446655440000"
                , "550e8400e29b41d4a716446655440000"
                , "{550e8400-e29b-41d4-a716-446655440000}"
                , "URN:UUID:550e8400-e29b-41d4-a716-446655440000"
                , "  550e8400-e29b-41d4-a716-446655440000  "
                , "550e8400-e29b41d4-a716446655440000"
                ]
        it "rejects malformed input" $
            mapM_
                (\value -> parse value `shouldSatisfy` isLeft)
                [ ""
                , "not-a-uuid"
                , "550e8400-e29b-41d4-a716"
                , "550e8400-e29b-41d4-a716-4466554400001234"
                , "550e8400-e29b-41d4-a716-44665544gggg"
                , "{550e8400-e29b-41d4-a716-446655440000"
                , "550e8400-e29b-41d4-a716-446655440000}"
                ]
        it "validates without throwing" $ do
            isValid (render sample) `shouldBe` True
            isValid "not-a-uuid" `shouldBe` False

    describe "metadata and constants" $ do
        it "reads versions and all variants" $ do
            uuidVersion sample `shouldBe` 4
            uuidVariant sample `shouldBe` "rfc4122"
            variantOf 0x00 `shouldBe` "ncs"
            variantOf 0x80 `shouldBe` "rfc4122"
            variantOf 0xc0 `shouldBe` "microsoft"
            variantOf 0xe0 `shouldBe` "reserved"
        it "provides ordered nil and max UUIDs" $ do
            isNil nilUUID `shouldBe` True
            isMax nilUUID `shouldBe` False
            isMax maxUUID `shouldBe` True
            isNil maxUUID `shouldBe` False
            nilUUID `shouldSatisfy` (< maxUUID)
        it "provides the four RFC namespaces" $ do
            render namespaceDNS `shouldBe` "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
            render namespaceURL `shouldBe` "6ba7b811-9dad-11d1-80b4-00c04fd430c8"
            render namespaceOID `shouldBe` "6ba7b812-9dad-11d1-80b4-00c04fd430c8"
            render namespaceX500 `shouldBe` "6ba7b814-9dad-11d1-80b4-00c04fd430c8"
            map uuidVersion [namespaceDNS, namespaceURL, namespaceOID, namespaceX500]
                `shouldBe` replicate 4 1

    describe "name-based UUIDs" $ do
        it "matches the v3 DNS python.org RFC vector" $ do
            render (v3 namespaceDNS "python.org") `shouldBe` "6fa459ea-ee8a-3ca4-894e-db77e160355e"
            uuidVersion (v3 namespaceDNS "python.org") `shouldBe` 3
            uuidVariant (v3 namespaceDNS "python.org") `shouldBe` "rfc4122"
        it "matches v5 DNS and URL vectors" $ do
            render (v5 namespaceDNS "python.org") `shouldBe` "886313e1-3b8a-5372-9b90-0c9aee199e5d"
            render (v5 namespaceURL "http://www.python.org/") `shouldBe` "c2a8cbf8-d0f1-5ef4-9740-c3faec8ab1a0"
        it "is deterministic and incorporates names and namespaces" $ do
            v5 namespaceDNS "example.com" `shouldBe` v5 namespaceDNS "example.com"
            v5 namespaceDNS "example.com" `shouldNotBe` v5 namespaceDNS "example.org"
            v5 namespaceDNS "example.com" `shouldNotBe` v5 namespaceURL "example.com"
            v3 namespaceDNS "python.org" `shouldNotBe` v5 namespaceDNS "python.org"
        it "encodes empty and Unicode names as UTF-8" $ do
            uuidVersion (v5 namespaceDNS "") `shouldBe` 5
            render (v5 namespaceURL "https://\x4f8b\x3048.jp/")
                `shouldBe` "eccf5197-3987-5080-845b-82d9b4c8af77"
            uuidVariant (v5 namespaceURL "https://\x4f8b\x3048.jp/") `shouldBe` "rfc4122"

    describe "random and time UUIDs" $ do
        it "generates unique RFC v4 UUIDs" $ do
            values <- sequence (replicate 100 v4)
            map uuidVersion values `shouldSatisfy` all (== 4)
            map uuidVariant values `shouldSatisfy` all (== "rfc4122")
            length (nub values) `shouldBe` 100
            map (length . uuidBytes) values `shouldSatisfy` all (== 16)
        it "generates unique RFC v1 UUIDs with multicast nodes" $ do
            values <- sequence (replicate 20 v1)
            map uuidVersion values `shouldSatisfy` all (== 1)
            map uuidVariant values `shouldSatisfy` all (== "rfc4122")
            length (nub values) `shouldBe` 20
            map ((.&. 0x01) . (!! 10) . uuidBytes) values `shouldSatisfy` all (== 1)
        it "generates unique time-bearing RFC v7 UUIDs" $ do
            first <- v7
            before <- currentUnixMilliseconds
            threadDelay 3000
            second <- v7
            after <- currentUnixMilliseconds
            values <- sequence (replicate 50 v7)
            uuidVersion first `shouldBe` 7
            uuidVariant first `shouldBe` "rfc4122"
            uuidTimestamp first `shouldSatisfy` (<= uuidTimestamp second)
            uuidTimestamp second `shouldSatisfy` (\value -> value >= before && value <= after + 1)
            length (nub values) `shouldBe` 50
  where
    variantOf byte = uuidVariant (expectRight (uuidFromBytes (replicate 8 0 ++ [byte] ++ replicate 7 0)))

expectRight :: Either UUIDError UUID -> UUID
expectRight result =
    case result of
        Right value -> value
        Left err -> error (errorMessage err)

uuidTimestamp :: UUID -> Integer
uuidTimestamp = foldl (\value byte -> value `shiftL` 8 + fromIntegral byte) 0 . take 6 . uuidBytes

currentUnixMilliseconds :: IO Integer
currentUnixMilliseconds = floor . (* 1000) <$> getPOSIXTime
