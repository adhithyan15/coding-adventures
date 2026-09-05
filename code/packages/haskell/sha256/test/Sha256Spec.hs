module Sha256Spec (spec) where

import Control.Monad (forM_)
import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BS8
import Data.Char (ord)
import Data.List (foldl')
import Data.Word (Word8)
import Sha256
import Sha256.Internal (checkedAdvanceBytes, maxSha256MessageBytes)
import Test.Hspec hiding (context)

spec :: Spec
spec = describe "Sha256" $ do
    it "hashes abc to the FIPS vector" $ do
        sha256Hex (ascii "abc")
            `shouldBe` "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

    it "hashes the empty string" $ do
        sha256Hex []
            `shouldBe` "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

    it "hashes the multi-block FIPS vector" $ do
        sha256Hex (ascii "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")
            `shouldBe` "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"

    it "returns a 32-byte digest" $ do
        length (sha256 (ascii "coding adventures")) `shouldBe` 32

    describe "incremental API" $ do
        it "finalizes the initialized context to the empty FIPS vector" $ do
            sha256FinalizeHex sha256Init
                `shouldBe` "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

        it "hashes exact bytes across multiple updates" $ do
            incrementalHex [BS8.pack "a", BS8.pack "bc"]
                `shouldBe` "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

        it "treats empty updates as identity" $ do
            incrementalHex [BS.empty, BS8.pack "a", BS.empty, BS8.pack "bc", BS.empty]
                `shouldBe` "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

        it "hashes binary bytes without text conversion" $ do
            incrementalHex [BS.pack [0x00, 0xff, 0x01, 0x80]]
                `shouldBe` "edc81f7e4ee358fb91e94bd9bd74079c3dcba36f40f2c8a36e7ae0567afecc8f"

        forM_ boundaryVectors $ \(byteCount, expected) ->
            it ("matches the independent vector at " ++ show byteCount ++ " bytes") $ do
                incrementalHex [payload byteCount] `shouldBe` expected

        it "is independent of every selected update split" $ do
            let bytes = payload 9000
                expected = "4b81efbd205e7fb4e42bc0d72d9d7413642298735289d35a74c1755883bcc45c"
            forM_ splitPoints $ \splitPoint ->
                incrementalHex [BS.take splitPoint bytes, BS.drop splitPoint bytes]
                    `shouldBe` expected

        it "supports byte-at-a-time updates without retaining message history" $ do
            let bytes = payload 8193
                chunks = map BS.singleton (BS.unpack bytes)
            incrementalHex chunks
                `shouldBe` "7e3691790cd64b19d4edb1a80e988214515abeb53aa0f34ffbfe4b4bf405d120"

        it "finalizes repeatedly and permits updates after finalization" $ do
            let context = sha256Update sha256Init (BS8.pack "abc")
            sha256FinalizeHex context
                `shouldBe` "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            sha256FinalizeHex context
                `shouldBe` "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            sha256FinalizeHex (sha256Update context (BS8.pack "x"))
                `shouldBe` "7571ce1f8e21c6b13dd7ec2c5ec7c9e4dd9852e209869511853f2f1f74b17927"

        it "copies immutable contexts for independent branches" $ do
            let base = sha256Update sha256Init (BS8.pack "common")
                left = sha256Update (sha256Copy base) (BS8.pack "left")
                right = sha256Update (sha256Copy base) (BS8.pack "right")
            sha256FinalizeHex left
                `shouldBe` "deb5e06581ed685ee3934d3f6f34af5ffdd803151401ef677c81ea44af8a653e"
            sha256FinalizeHex right
                `shouldBe` "201c220c620c540c6adee783f95a78f9e164b4e24abc8f613ba98514f22c668d"

        it "matches the million-a NIST vector in two bounded chunks" $ do
            let chunk = BS8.replicate 500000 'a'
            incrementalHex [chunk, chunk]
                `shouldBe` "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"

        it "returns a 32-byte digest and lowercase 64-character hex" $ do
            let context = sha256Update sha256Init (BS8.pack "coding adventures")
                digestBytes = sha256Finalize context
                rendered = sha256FinalizeHex context
            BS.length digestBytes `shouldBe` 32
            length rendered `shouldBe` 64
            rendered `shouldSatisfy` all (\character -> character `elem` ['0' .. '9'] ++ ['a' .. 'f'])

    describe "message length domain" $ do
        it "accepts the largest whole-byte FIPS message length" $ do
            checkedAdvanceBytes (maxSha256MessageBytes - 1) 1
                `shouldBe` Just maxSha256MessageBytes

        it "rejects a byte count whose bit length would reach 2^64" $ do
            checkedAdvanceBytes maxSha256MessageBytes 1 `shouldBe` Nothing

        it "rejects an already out-of-domain byte count" $ do
            checkedAdvanceBytes maxBound 0 `shouldBe` Nothing

ascii :: String -> [Word8]
ascii = map (fromIntegral . ord)

incrementalHex :: [BS.ByteString] -> String
incrementalHex = sha256FinalizeHex . foldl' sha256Update sha256Init

payload :: Int -> BS.ByteString
payload byteCount =
    BS.pack [fromIntegral (index `mod` 251) | index <- [0 .. byteCount - 1]]

splitPoints :: [Int]
splitPoints = [55, 56, 63, 64, 65, 8191, 8192, 8193]

boundaryVectors :: [(Int, String)]
boundaryVectors =
    [ (55, "463eb28e72f82e0a96c0a4cc53690c571281131f672aa229e0d45ae59b598b59")
    , (56, "da2ae4d6b36748f2a318f23e7ab1dfdf45acdc9d049bd80e59de82a60895f562")
    , (63, "29af2686fd53374a36b0846694cc342177e428d1647515f078784d69cdb9e488")
    , (64, "fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108")
    , (65, "4bfd2c8b6f1eec7a2afeb48b934ee4b2694182027e6d0fc075074f2fabb31781")
    , (8191, "a7ff4cc384f150c0763c051418a0084ded32bfa5863717ab5f35d3f43a5ffe1c")
    , (8192, "25df2449b2e5a35fea14e02a7158e283801a1069c9f84631b9a9dacb2f809a7f")
    , (8193, "7e3691790cd64b19d4edb1a80e988214515abeb53aa0f34ffbfe4b4bf405d120")
    ]
