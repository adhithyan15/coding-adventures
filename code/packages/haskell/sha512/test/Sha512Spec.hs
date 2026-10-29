module Sha512Spec (spec) where

import Data.Char (ord)
import Data.List (isPrefixOf)
import Data.Word (Word8)
import Numeric (readHex)
import Sha512
import Test.Hspec

spec :: Spec
spec = describe "Sha512" $ do
    it "matches the bounded HF04 portable corpus" $ do
        rows <- fixtureRows
        mapM_ checkVector [row | row <- rows, "V|" `isPrefixOf` row]

    it "encodes the full 128-bit length field without allocating messages" $ do
        rows <- fixtureRows
        mapM_ checkLength [row | row <- rows, "L|" `isPrefixOf` row]

    it "hashes abc to the FIPS vector" $ do
        sha512Hex (ascii "abc")
            `shouldBe` "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"

    it "hashes the empty string" $ do
        sha512Hex []
            `shouldBe` "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"

    it "returns a 64-byte digest" $ do
        length (sha512 (ascii "coding adventures")) `shouldBe` 64

ascii :: String -> [Word8]
ascii = map (fromIntegral . ord)

fixtureRows :: IO [String]
fixtureRows = do
    raw <- readFile "../../../specs/fixtures/sha512-v1/cases.txt"
    length raw `shouldSatisfy` (\size -> size > 0 && size <= 4096)
    let rows = filter (\line -> case line of { first : _ -> first /= '#'; [] -> False }) (lines raw)
    length rows `shouldSatisfy` (\count -> count > 0 && count <= 64)
    pure rows

splitPipe :: String -> [String]
splitPipe [] = [""]
splitPipe value = case break (== '|') value of
    (field, []) -> [field]
    (field, _ : rest) -> field : splitPipe rest

hexInteger :: String -> Integer
hexInteger value = case readHex value of
    [(number, "")] -> number
    _ -> error "invalid bounded SHA-512 fixture hex"

decodeHex :: String -> [Word8]
decodeHex [] = []
decodeHex (a:b:rest) = fromIntegral (hexInteger [a, b]) : decodeHex rest
decodeHex _ = error "odd bounded SHA-512 fixture hex"

checkVector :: String -> Expectation
checkVector row = case splitPipe row of
    ["V", caseId, inputHex, _, expected] -> do
        length inputHex `shouldSatisfy` (<= 8192)
        length expected `shouldBe` 128
        let input = decodeHex inputHex
        length input `shouldSatisfy` (<= 4096)
        sha512Hex input `shouldBe` expected
        caseId `shouldSatisfy` (not . null)
    _ -> expectationFailure "malformed SHA-512 vector row"

checkLength :: String -> Expectation
checkLength row = case splitPipe row of
    ["L", caseId, highHex, lowHex, expected] -> do
        let byteCount = hexInteger highHex * 2 ^ (64 :: Int) + hexInteger lowHex
        case (expected, encodeBitLength byteCount) of
            ("ERROR", Left message) -> message `shouldBe` "sha512: message length must be less than 2^128 bits"
            ("ERROR", Right _) -> expectationFailure (caseId ++ " accepted overflow")
            (_, Right encoded) -> encoded `shouldBe` decodeHex expected
            (_, Left message) -> expectationFailure (caseId ++ ": " ++ message)
    _ -> expectationFailure "malformed SHA-512 length row"
