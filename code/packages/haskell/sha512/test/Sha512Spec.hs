module Sha512Spec (spec) where

import Control.Exception (finally)
import qualified Data.ByteString as BS
import Data.Char (isDigit, ord)
import Data.Either (isLeft)
import Data.List (isPrefixOf)
import Data.Word (Word8)
import Numeric (readHex)
import Sha512
import System.Directory (getTemporaryDirectory, removeFile)
import System.IO (IOMode(ReadMode), hClose, openBinaryTempFile, withBinaryFile)
import Test.Hspec

spec :: Spec
spec = describe "Sha512" $ do
    it "matches the bounded HF04 portable corpus" $ do
        rows <- fixtureRows
        mapM_ checkVector [row | row <- rows, "V|" `isPrefixOf` row]

    it "encodes the full 128-bit length field without allocating messages" $ do
        rows <- fixtureRows
        mapM_ checkLength [row | row <- rows, "L|" `isPrefixOf` row]

    it "rejects malformed fixture rows" $ do
        let digest = replicate 128 '0'
        mapM_ (\row -> parseFixture row `shouldSatisfy` isLeft)
            ["X|id|||", "V|UPPER|00|1|" ++ digest, "V|bad|0|1|" ++ digest,
             "V|bad|00|2|" ++ digest, "L|bad|000000000000000G|0000000000000000|ERROR",
             "L|bad|0000000000000000|0000000000000000|ABC"]

    it "rejects an oversized fixture through the real file reader" $ do
        temporary <- getTemporaryDirectory
        (path, handle) <- openBinaryTempFile temporary "sha512-fixture.txt"
        (do
            BS.hPut handle (BS.replicate 4097 0)
            hClose handle
            result <- readFixtureRows path
            result `shouldSatisfy` isLeft
          ) `finally` removeFile path

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
    result <- readFixtureRows "../../../specs/fixtures/sha512-v1/cases.txt"
    rows <- case result of
        Left message -> expectationFailure message >> pure []
        Right parsed -> pure parsed
    length [row | row <- rows, "V|" `isPrefixOf` row] `shouldBe` 9
    length [row | row <- rows, "L|" `isPrefixOf` row] `shouldBe` 6
    pure rows

readFixtureRows :: FilePath -> IO (Either String [String])
readFixtureRows path = withBinaryFile path ReadMode $ \handle -> do
    raw <- BS.hGet handle 4097
    if BS.length raw > 4096
        then pure (Left "fixture byte size or encoding outside bounds")
        else pure (parseFixture (map (toEnum . fromIntegral) (BS.unpack raw)))

parseFixture :: String -> Either String [String]
parseFixture raw
    | null raw || length raw > 4096 || any ((> 127) . ord) raw = Left "fixture byte size or encoding outside bounds"
    | null rows || length rows > 64 = Left "fixture row count outside bounds"
    | otherwise = mapM validateRow rows
  where rows = filter (\line -> case line of { first : _ -> first /= '#'; [] -> False }) (lines raw)

validateRow :: String -> Either String String
validateRow row = case splitPipe row of
    ["V", caseId, inputHex, chunks, expected]
        | not (validId caseId) || length inputHex > 8192 || odd (length inputHex) || not (all lowerHex inputHex)
            || length expected /= 128 || not (all lowerHex expected) -> Left "malformed vector row"
        | byteCount > 4096 || (byteCount == 0) /= null chunks -> Left "malformed vector size"
        | otherwise -> case parseChunks chunks of
            Just values | length values <= 64 && sum values == byteCount -> Right row
            _ -> Left "malformed chunks"
      where byteCount = length inputHex `div` 2
    ["L", caseId, highHex, lowHex, expected]
        | validId caseId && validHex 16 highHex && validHex 16 lowHex && (expected == "ERROR" || validHex 32 expected) -> Right row
        | otherwise -> Left "malformed length row"
    _ -> Left "unknown or malformed fixture row"

lowerHex :: Char -> Bool
lowerHex value = isDigit value || value >= 'a' && value <= 'f'

validHex :: Int -> String -> Bool
validHex size value = length value == size && all lowerHex value

validId :: String -> Bool
validId value = not (null parts) && all (\part -> not (null part) && all (\c -> isDigit c || c >= 'a' && c <= 'z') part) parts
  where parts = splitOn '-' value

splitOn :: Char -> String -> [String]
splitOn _ [] = [""]
splitOn delimiter value = case break (== delimiter) value of
    (field, []) -> [field]
    (field, _ : rest) -> field : splitOn delimiter rest

parseChunks :: String -> Maybe [Int]
parseChunks "" = Just []
parseChunks value = mapM parseChunk (splitOn ',' value)
  where
    parseChunk raw@(first : _)
        | first == '0' || length raw > 4 || not (all isDigit raw) = Nothing
        | otherwise = let size = read raw in if size <= 4096 then Just size else Nothing
    parseChunk [] = Nothing

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
