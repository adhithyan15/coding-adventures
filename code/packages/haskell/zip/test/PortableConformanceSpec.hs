{-# LANGUAGE OverloadedStrings #-}

module PortableConformanceSpec (spec) where

import Control.Exception (IOException, try)
import Data.Aeson
import Data.Bits (shiftR)
import Data.ByteString (ByteString)
import qualified Data.ByteString as BS
import Data.Char (toLower)
import Data.List (find)
import Data.Maybe (fromMaybe)
import Data.Word (Word16, Word32)
import Numeric (readHex, showHex)
import System.Directory (doesFileExist, findExecutable, getCurrentDirectory)
import System.Exit (ExitCode(..))
import System.FilePath ((</>), takeDirectory)
import System.Process (readProcessWithExitCode)
import Test.Hspec

import Zip

data Fixture = Fixture
    { fixtureSchemaVersion :: Int
    , fixtureProfile :: String
    , fixtureLimits :: Limits
    , fixtureErrorIds :: [String]
    , fixtureCases :: [FixtureCase]
    } deriving (Show)

data Limits = Limits
    { limitsDefaultMaxOutput :: Int
    , limitsHardMaxOutput :: Int
    } deriving (Show)

data FixtureCase = FixtureCase
    { caseId :: String
    , caseOperation :: String
    , caseInputHex :: Maybe String
    , caseMaxOutput :: Maybe Int
    , caseExpected :: Expected
    , caseChunksHex :: Maybe [String]
    , caseInitialCrc32Hex :: Maybe String
    } deriving (Show)

data Expected = Expected
    { expectedOutput :: Maybe OutputBytes
    , expectedBytesConsumed :: Maybe Int
    , expectedErrorId :: Maybe String
    , expectedCrc32Hex :: Maybe String
    } deriving (Show)

data OutputBytes = OutputBytes
    { outputHex :: Maybe String
    , outputRepeatHex :: Maybe String
    , outputCount :: Maybe Int
    } deriving (Show)

instance FromJSON Fixture where
    parseJSON = withObject "Fixture" $ \o -> Fixture
        <$> o .: "schema_version"
        <*> o .: "profile"
        <*> o .: "limits"
        <*> o .: "error_ids"
        <*> o .: "cases"

instance FromJSON Limits where
    parseJSON = withObject "Limits" $ \o -> Limits
        <$> o .: "default_max_output"
        <*> o .: "hard_max_output"

instance FromJSON FixtureCase where
    parseJSON = withObject "FixtureCase" $ \o -> FixtureCase
        <$> o .: "id"
        <*> o .: "operation"
        <*> o .:? "input_hex"
        <*> o .:? "max_output"
        <*> o .: "expected"
        <*> o .:? "chunks_hex"
        <*> o .:? "initial_crc32_hex"

instance FromJSON Expected where
    parseJSON = withObject "Expected" $ \o -> Expected
        <$> o .:? "output"
        <*> o .:? "bytes_consumed"
        <*> o .:? "error_id"
        <*> o .:? "crc32_hex"

instance FromJSON OutputBytes where
    parseJSON = withObject "OutputBytes" $ \o -> OutputBytes
        <$> o .:? "hex"
        <*> o .:? "repeat_hex"
        <*> o .:? "count"

expectedErrorCodes :: [String]
expectedErrorCodes =
    [ "invalid-output-limit"
    , "unexpected-eof"
    , "reserved-block-type"
    , "stored-length-mismatch"
    , "huffman-oversubscribed"
    , "incomplete-code-length-tree"
    , "incomplete-literal-length-tree"
    , "incomplete-distance-tree"
    , "repeat-without-previous"
    , "repeat-overrun"
    , "invalid-literal-length-symbol"
    , "reserved-distance-symbol"
    , "invalid-back-reference"
    , "output-limit-exceeded"
    ]

spec :: Spec
spec = describe "portable ZIP-owned raw RFC 1951 profile" $ do
    fixture <- runIO loadFixture

    it "consumes all 34 language-neutral fixture cases" $ do
        fixtureSchemaVersion fixture `shouldBe` 1
        fixtureProfile fixture `shouldBe` "zip-owned-raw-rfc1951-v1"
        limitsDefaultMaxOutput (fixtureLimits fixture) `shouldBe` rawInflateMaxOutput
        limitsHardMaxOutput (fixtureLimits fixture) `shouldBe` rawInflateMaxOutput
        fixtureErrorIds fixture `shouldBe` expectedErrorCodes
        rawInflateErrorCodes `shouldBe` expectedErrorCodes
        length (fixtureCases fixture) `shouldBe` 34
        mapM_ runFixtureCase (fixtureCases fixture)

    it "accepts a foreign dynamic stream through the ZIP container" $ do
        testCase <- requireCase fixture "zip-raw-v1-inflate-dynamic-foreign"
        compressed <- hexBytes (required "input_hex" (caseInputHex testCase))
        plain <- materialize (required "output" (expectedOutput (caseExpected testCase)))
        readEntry (rawZip "dynamic.bin" compressed plain (BS.length plain)) "dynamic.bin"
            `shouldBe` Right plain

    it "rejects a compressed-payload suffix cavity" $ do
        testCase <- requireCase fixture "zip-raw-v1-inflate-dynamic-foreign"
        compressed <- hexBytes (required "input_hex" (caseInputHex testCase))
        plain <- materialize (required "output" (expectedOutput (caseExpected testCase)))
        readEntry (rawZip "dynamic.bin" (compressed <> BS.pack [0xde, 0xad]) plain (BS.length plain)) "dynamic.bin"
            `shouldBe` Left "zip: compressed payload contains trailing bytes"

    it "rejects declared uncompressed-size mismatches" $ do
        testCase <- requireCase fixture "zip-raw-v1-inflate-dynamic-foreign"
        compressed <- hexBytes (required "input_hex" (caseInputHex testCase))
        plain <- materialize (required "output" (expectedOutput (caseExpected testCase)))
        readEntry (rawZip "dynamic.bin" compressed plain (BS.length plain + 1)) "dynamic.bin"
            `shouldBe` Left "zip: uncompressed size does not match the directory"

    it "rejects stored entries whose directory sizes disagree" $ do
        let plain = "stored bytes"
            archive = rawZipWithMethod 0 "stored.bin" plain plain (BS.length plain + 1)
        readEntry archive "stored.bin"
            `shouldBe` Left "zip: uncompressed size does not match the directory"

    it "supports a foreign stream using the full 32 KiB history window" $ do
        let prefix = BS.pack [fromIntegral ((i * 73 + i `div` 251) `mod` 256) | i <- [0 .. 32767 :: Int]]
            expected = prefix <> prefix
        encoded <- pythonRaw "compress" expected
        rawInflate encoded (BS.length expected) `shouldBe` Right expected

    it "preserves the historical wrappers and validates direct limits" $ do
        let payload = "historical wrapper compatibility"
        deflateDecompress (deflateCompress payload) `shouldBe` Right payload
        rawDeflate payload `shouldBe` deflateCompress payload
        rawInflateCounted (BS.pack [0x01, 0x00, 0x00, 0xff, 0xff]) (-1)
            `shouldBe` Left InvalidOutputLimit
        rawInflateCounted (BS.pack [0x01, 0x00, 0x00, 0xff, 0xff]) (rawInflateMaxOutput + 1)
            `shouldBe` Left InvalidOutputLimit

runFixtureCase :: FixtureCase -> IO ()
runFixtureCase testCase =
    case caseOperation testCase of
        "inflate" -> do
            input <- hexBytes (required "input_hex" (caseInputHex testCase))
            expected <- materialize (required "output" (expectedOutput (caseExpected testCase)))
            let limit = fromMaybe rawInflateMaxOutput (caseMaxOutput testCase)
                consumed = required "bytes_consumed" (expectedBytesConsumed (caseExpected testCase))
            rawInflateCounted input limit
                `shouldBe` Right (RawInflateResult expected consumed)
            rawInflate input limit `shouldBe` Right expected
        "inflate-error" -> do
            input <- hexBytes (required "input_hex" (caseInputHex testCase))
            let limit = fromMaybe rawInflateMaxOutput (caseMaxOutput testCase)
                expectedCode = required "error_id" (expectedErrorId (caseExpected testCase))
            case rawInflateCounted input limit of
                Left err -> do
                    rawInflateErrorCode err `shouldBe` expectedCode
                    show err `shouldBe` expectedCode
                Right _ -> expectationFailure (caseId testCase ++ ": expected " ++ expectedCode)
        "deflate-interoperability" -> do
            input <- hexBytes (required "input_hex" (caseInputHex testCase))
            expected <- materialize (required "output" (expectedOutput (caseExpected testCase)))
            decoded <- pythonRaw "decompress" (rawDeflate input)
            decoded `shouldBe` expected
        "crc32" -> do
            chunks <- mapM hexBytes (required "chunks_hex" (caseChunksHex testCase))
            let initial = maybe 0 parseHexWord32 (caseInitialCrc32Hex testCase)
                actual = foldl (flip crc32) initial chunks
                expected = map toLower (required "crc32_hex" (expectedCrc32Hex (caseExpected testCase)))
            padHex actual `shouldBe` expected
        operation -> expectationFailure (caseId testCase ++ ": unsupported operation " ++ operation)

loadFixture :: IO Fixture
loadFixture = do
    cwd <- getCurrentDirectory
    path <- findFixturePath cwd 8
    result <- eitherDecodeFileStrict' path
    case result of
        Left err -> fail ("fixture decode failed: " ++ err)
        Right fixture -> pure fixture

findFixturePath :: FilePath -> Int -> IO FilePath
findFixturePath start remaining = do
    let direct = start </> "code" </> "specs" </> "fixtures" </> "zip-raw-rfc1951-v1" </> "cases.json"
        fromCode = start </> "specs" </> "fixtures" </> "zip-raw-rfc1951-v1" </> "cases.json"
    directExists <- doesFileExist direct
    codeExists <- doesFileExist fromCode
    if directExists
        then pure direct
        else if codeExists
            then pure fromCode
            else if remaining <= 0 || takeDirectory start == start
                then fail "zip raw RFC 1951 fixture not found"
                else findFixturePath (takeDirectory start) (remaining - 1)

requireCase :: Fixture -> String -> IO FixtureCase
requireCase fixture wanted =
    case find ((== wanted) . caseId) (fixtureCases fixture) of
        Just testCase -> pure testCase
        Nothing -> fail ("fixture case not found: " ++ wanted)

materialize :: OutputBytes -> IO ByteString
materialize output =
    case outputHex output of
        Just value -> hexBytes value
        Nothing -> do
            repeated <- hexBytes (required "repeat_hex" (outputRepeatHex output))
            case BS.uncons repeated of
                Just (byte, rest) | BS.null rest ->
                    pure (BS.replicate (required "count" (outputCount output)) byte)
                _ -> fail "repeat_hex must contain exactly one byte"

hexBytes :: String -> IO ByteString
hexBytes [] = pure BS.empty
hexBytes (a:b:rest) =
    case readHex [a, b] of
        [(value, "")] -> BS.cons (fromIntegral (value :: Int)) <$> hexBytes rest
        _ -> fail "invalid fixture hex"
hexBytes _ = fail "fixture hex must contain whole bytes"

pythonRaw :: String -> ByteString -> IO ByteString
pythonRaw mode input = do
    pythonCandidate <- findExecutable "python"
    executable <- case pythonCandidate of
        Just path -> pure (Just path)
        Nothing -> findExecutable "python3"
    case executable of
        Nothing -> fail "python or python3 is required for the independent RFC 1951 oracle"
        Just pythonPath -> do
            let script = unlines
                    [ "import sys, zlib"
                    , "data = bytes.fromhex(sys.stdin.read().strip())"
                    , "if sys.argv[1] == 'compress':"
                    , "    codec = zlib.compressobj(9, zlib.DEFLATED, -15)"
                    , "    result = codec.compress(data) + codec.flush()"
                    , "else:"
                    , "    result = zlib.decompress(data, -15)"
                    , "sys.stdout.write(result.hex())"
                    ]
                inputHex = concatMap byteHex (BS.unpack input)
            outcome <- try (readProcessWithExitCode pythonPath ["-c", script, mode] (inputHex ++ "\n"))
                :: IO (Either IOException (ExitCode, String, String))
            case outcome of
                Left _ -> fail "python RFC 1951 oracle unavailable"
                Right (ExitSuccess, output, _) -> hexBytes (map toLower output)
                Right (ExitFailure _, _, _) -> fail "python RFC 1951 oracle failed"

byteHex :: (Integral a, Show a) => a -> String
byteHex value =
    let rendered = showHex value ""
    in if length rendered == 1 then '0' : rendered else rendered

padHex :: Word32 -> String
padHex value = replicate (8 - length rendered) '0' ++ rendered
  where
    rendered = showHex value ""

parseHexWord32 :: String -> Word32
parseHexWord32 value =
    case readHex value of
        [(parsed, "")] -> parsed
        _ -> error "invalid fixture CRC-32 hex"

required :: String -> Maybe a -> a
required _ (Just value) = value
required field Nothing = error ("fixture missing required field: " ++ field)

rawZip :: ByteString -> ByteString -> ByteString -> Int -> ByteString
rawZip = rawZipWithMethod 8

rawZipWithMethod :: Word16 -> ByteString -> ByteString -> ByteString -> Int -> ByteString
rawZipWithMethod method name compressed plain declaredSize = local <> central <> eocd
  where
    checksum = crc32 plain 0
    compressedSize = BS.length compressed
    local = BS.concat
        [ le32 0x04034b50, le16 20, le16 0, le16 method, le16 0, le16 0
        , le32 checksum, le32i compressedSize, le32i declaredSize
        , le16i (BS.length name), le16 0, name, compressed
        ]
    central = BS.concat
        [ le32 0x02014b50, le16 20, le16 20, le16 0, le16 method, le16 0, le16 0
        , le32 checksum, le32i compressedSize, le32i declaredSize
        , le16i (BS.length name), le16 0, le16 0, le16 0, le16 0, le32 0, le32 0, name
        ]
    eocd = BS.concat
        [ le32 0x06054b50, le16 0, le16 0, le16 1, le16 1
        , le32i (BS.length central), le32i (BS.length local), le16 0
        ]

le16i :: Int -> ByteString
le16i = le16 . fromIntegral

le32i :: Int -> ByteString
le32i = le32 . fromIntegral

le16 :: Word16 -> ByteString
le16 value = BS.pack [fromIntegral value, fromIntegral (value `shiftR` 8)]

le32 :: Word32 -> ByteString
le32 value = BS.pack
    [ fromIntegral value
    , fromIntegral (value `shiftR` 8)
    , fromIntegral (value `shiftR` 16)
    , fromIntegral (value `shiftR` 24)
    ]
