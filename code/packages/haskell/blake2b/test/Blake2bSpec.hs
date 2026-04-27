-- | Test suite for the pure-Haskell BLAKE2b implementation.
--
-- All expected values are pre-computed from Python's hashlib.blake2b and
-- mirrored across every sibling language in the monorepo.  Matching
-- these KATs proves we implement RFC 7693 correctly.
module Blake2bSpec (spec) where

import Blake2b
import Data.Char (ord)
import Data.Word (Word8)
import Control.Exception (evaluate)
import Test.Hspec

spec :: Spec
spec = describe "Blake2b" $ do

    -- --------------------------------------------------------------
    -- Canonical vectors
    -- --------------------------------------------------------------

    describe "canonical vectors" $ do

        it "hashes the empty string to the RFC digest" $
            blake2bHex []
                `shouldBe`
                    "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419\
                    \d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"

        it "hashes 'abc' to the RFC 7693 Appendix A digest" $
            blake2bHex (ascii "abc")
                `shouldBe`
                    "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1\
                    \7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"

        it "hashes the quick brown fox" $
            blake2bHex (ascii "The quick brown fox jumps over the lazy dog")
                `shouldBe`
                    "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673\
                    \f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918"

        it "truncates to digest_size=32 for the empty string" $
            blake2bHexWith defaultParams {digestSize = 32} []
                `shouldBe` "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8"

        it "matches the keyed long vector (key=1..64, data=0..255)" $
            let k = bytesFromRange 1 65
                d = bytesFromRange 0 256
             in blake2bHexWith defaultParams {key = k} d
                    `shouldBe`
                        "402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927\
                        \ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3"

    -- --------------------------------------------------------------
    -- Block-boundary sizes (exercises final-block flagging)
    -- --------------------------------------------------------------

    describe "block-boundary sizes" $ do

        let sizeData n = take n [fromIntegral ((i * 7 + 3) `mod` 256) :: Word8 | i <- [0 ..]]
        let katFor (size, want) =
                it ("size " ++ show size) $
                    blake2bHex (sizeData size) `shouldBe` want

        mapM_
            katFor
            [ (0, "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce")
            , (1, "4fe4da61bcc756071b226843361d74944c72245d23e8245ea678c13fdcd7fe2ae529cf999ad99cc24f7a73416a18ba53e76c0afef83b16a568b12fbfc1a2674d")
            , (63, "70b2a0e6daecac22c7a2df82c06e3fc0b4c66bd5ef8098e4ed54e723b393d79ef3bceba079a01a14c6ef2ae2ed1171df1662cd14ef38e6f77b01c7f48144dd09")
            , (64, "3db7bb5c40745f0c975ac6bb8578f590e2cd2cc1fc6d13533ef725325c9fddff5cca24e7a591a0f6032a24fad0e09f6df873c4ff314628391f78df7f09cb7ed7")
            , (65, "149c114a3e8c6e06bafee27c9d0de0e39ef28294fa0d9f81876dcceb10bb41101e256593587e46b844819ed7ded90d56c0843df06c95d1695c3de635cd7a888e")
            , (127, "71546bbf9110ad184cc60f2eb120fcfd9b4dbbca7a7f1270045b8a23a6a4f4330f65c1f030dd2f5fabc6c57617242c37cf427bd90407fac5b9deffd3ae888c39")
            , (128, "2d9e329f42afa3601d646692b81c13e87fcaff5bf15972e9813d7373cb6d181f9599f4d513d4af4fd6ebd37497aceb29aba5ee23ed764d8510b552bd088814fb")
            , (129, "47889df9eb4d717afc5019df5c6a83df00a0b8677395e078cd5778ace0f338a618e68b7d9afb065d9e6a01ccd31d109447e7fae771c3ee3e105709194122ba2b")
            , (255, "1a5199ac66a00e8a87ad1c7fbad30b33137dd8312bf6d98602dacf8f40ea2cb623a7fbc63e5a6bfa434d337ae7da5ca1a52502a215a3fe0297a151be85d88789")
            , (256, "91019c558584980249ca43eceed27e19f1c3c24161b93eed1eee2a6a774f60bf8a81b43750870bee1698feac9c5336ae4d5c842e7ead159bf3916387e8ded9ae")
            , (257, "9f1975efca45e7b74b020975d4d2c22802906ed8bfefca51ac497bd23147fc8f303890d8e5471ab6caaa02362e831a9e8d3435279912ccd4842c7806b096c348")
            , (1024, "eddc3f3af9392eff065b359ce5f2b28f71e9f3a3a50e60ec27787b9fa623094d17b046c1dfce89bc5cdfc951b95a9a9c05fb8cc2361c905db01dd237fe56efb3")
            ]

    -- --------------------------------------------------------------
    -- Variable digest sizes
    -- --------------------------------------------------------------

    describe "variable digest sizes" $ do
        let foxData = ascii "The quick brown fox jumps over the lazy dog"

        let runKat (ds, want) = it ("digest_size " ++ show ds) $ do
                let out = blake2bWith defaultParams {digestSize = ds} foxData
                length out `shouldBe` ds
                hexOf out `shouldBe` want

        mapM_
            runKat
            [ (1, "b5")
            , (16, "249df9a49f517ddcd37f5c897620ec73")
            , (20, "3c523ed102ab45a37d54f5610d5a983162fde84f")
            , (32, "01718cec35cd3d796dd00020e0bfecb473ad23457d063b75eff29c0ffa2e58a9")
            , (48, "b7c81b228b6bd912930e8f0b5387989691c1cee1e65aade4da3b86a3c9f678fc8018f6ed9e2906720c8d2a3aeda9c03d")
            , (64, "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918")
            ]

    -- --------------------------------------------------------------
    -- Keyed variants
    -- --------------------------------------------------------------

    describe "keyed variants" $ do
        let body = ascii "secret message body"

        let runKat (klen, want) = it ("keyed keyLen " ++ show klen) $
                blake2bHexWith
                    defaultParams {digestSize = 32, key = bytesFromRange 1 (klen + 1)}
                    body
                    `shouldBe` want

        mapM_
            runKat
            [ (1, "affd4e429aa2fb18da276f6ecff16f7d048769cacefe1a7ac75184448e082422")
            , (16, "5f8510d05dac42e8b6fc542af93f349d41ae4ebaf5cecae4af43fae54c7ca618")
            , (32, "88a78036d5890e91b5e3d70ba4738d2be302b76e0857d8ee029dc56dfa04fe67")
            , (64, "df7eab2ec9135ab8c58f48c288cdc873bac245a7fa46ca9f047cab672bd1eabb")
            ]

    -- --------------------------------------------------------------
    -- Salt + personal
    -- --------------------------------------------------------------

    describe "salt + personal" $
        it "matches the shared cross-language KAT" $
            blake2bHexWith
                defaultParams {salt = bytesFromRange 0 16, personal = bytesFromRange 16 32}
                (ascii "parameterized hash")
                `shouldBe`
                    "a2185d648fc63f3d363871a76360330c9b238af5466a20f94bb64d363289b95d\
                    \a0453438eea300cd6f31521274ec001011fa29e91a603fabf00f2b454e30bf3d"

    -- --------------------------------------------------------------
    -- Validation
    -- --------------------------------------------------------------

    describe "validation" $ do
        it "rejects digest_size 0" $
            evaluate (blake2bWith defaultParams {digestSize = 0} [])
                `shouldThrow` anyErrorCall

        it "rejects digest_size 65" $
            evaluate (blake2bWith defaultParams {digestSize = 65} [])
                `shouldThrow` anyErrorCall

        it "rejects key length > 64" $
            evaluate (blake2bWith defaultParams {key = replicate 65 0} [])
                `shouldThrow` anyErrorCall

        it "rejects salt of wrong length" $
            evaluate (blake2bWith defaultParams {salt = replicate 8 0} [])
                `shouldThrow` anyErrorCall

        it "rejects personal of wrong length" $
            evaluate (blake2bWith defaultParams {personal = replicate 20 0} [])
                `shouldThrow` anyErrorCall

        it "accepts a 64-byte key" $
            length (blake2bWith defaultParams {key = replicate 64 0x41} (ascii "x"))
                `shouldBe` 64

-- ------------------------------------------------------------------
-- Helpers
-- ------------------------------------------------------------------

ascii :: String -> [Word8]
ascii = map (fromIntegral . ord)

bytesFromRange :: Int -> Int -> [Word8]
bytesFromRange start stop = [fromIntegral (i `mod` 256) | i <- [start .. stop - 1]]

hexOf :: [Word8] -> String
hexOf = concatMap renderByte
  where
    renderByte b =
        let hi = b `div` 16
            lo = b `mod` 16
         in [hexDigit hi, hexDigit lo]
    hexDigit n
        | n < 10 = toEnum (fromIntegral n + fromEnum '0')
        | otherwise = toEnum (fromIntegral n - 10 + fromEnum 'a')
