module ZstdSpec (spec) where

import qualified Data.ByteString as BS
import qualified Data.ByteString.Char8 as BSC
import Data.Bits ((.&.), (.|.), shiftL, shiftR)
import Data.Char (digitToInt)
import Data.Word (Word32)
import Test.Hspec
import Zstd

spec :: Spec
spec = do
    describe "compress and decompress" $ do
        it "round trips empty input and emits the canonical empty frame" $ do
            compressed <- expectRight (compress BS.empty)
            compressed
                `shouldBe` BS.pack
                    [ 0x28, 0xB5, 0x2F, 0xFD, 0xE0
                    , 0, 0, 0, 0, 0, 0, 0, 0
                    , 1, 0, 0
                    ]
            decompress compressed `shouldBe` Right BS.empty

        it "round trips representative single bytes" $
            mapM_ roundTrip [BS.singleton 0, BS.singleton 0x42, BS.singleton 0xFF]

        it "uses a raw block for all byte values" $ do
            let input = BS.pack [0 .. 255]
            compressed <- expectRight (compress input)
            decompress compressed `shouldBe` Right input
            blockTypes compressed `shouldBe` [0]

        it "uses RLE blocks for uniform byte runs" $ do
            mapM_
                (\value -> do
                    let input = BS.replicate 1024 value
                    compressed <- expectRight (compress input)
                    decompress compressed `shouldBe` Right input
                    BS.length compressed `shouldSatisfy` (< 30)
                    blockTypes compressed `shouldBe` [1]
                )
                [0, 0x41, 0xFF]

        it "uses a compressed block for repetitive prose" $ do
            let input = BSC.pack (concat (replicate 25 "the quick brown fox jumps over the lazy dog "))
            compressed <- expectRight (compress input)
            decompress compressed `shouldBe` Right input
            BS.length compressed `shouldSatisfy` (< BS.length input * 4 `div` 5)
            blockTypes compressed `shouldBe` [2]

        it "round trips deterministic binary data" $ do
            let input = pseudoRandomBytes 512
            roundTrip input

        it "emits multiple RLE blocks across the 128 KiB boundary" $ do
            let input = BS.replicate (200 * 1024) 0x78
            compressed <- expectRight (compress input)
            decompress compressed `shouldBe` Right input
            blockTypes compressed `shouldBe` [1, 1]
            BS.length compressed `shouldSatisfy` (< 50)

        it "compresses repeated-distance patterns efficiently" $ do
            let segment = BS.replicate 128 0x58 <> BSC.pack "ABCDEFGH"
                input = BSC.pack "ABCDEFGH" <> BS.concat (replicate 10 segment)
            compressed <- expectRight (compress input)
            decompress compressed `shouldBe` Right input
            BS.length compressed `shouldSatisfy` (< BS.length input * 7 `div` 10)

        it "is deterministic" $ do
            let input = BSC.pack (concat (replicate 50 "hello zstd world! "))
            compress input `shouldBe` compress input

        it "matches the established cross-language compressed vector" $
            compress (BSC.pack (concat (replicate 10 "hello zstd world! ")))
                `shouldBe` Right
                    (hexBytes "28b52ffde0b400000000000000dd00009068656c6c6f207a73746420776f726c6421200100f50100402930")

    describe "standard frame forms" $ do
        it "decodes a hand-crafted raw frame" $
            decompress
                ( BS.pack
                    [ 0x28, 0xB5, 0x2F, 0xFD, 0x20, 0x05
                    , 0x29, 0, 0, 0x68, 0x65, 0x6C, 0x6C, 0x6F
                    ]
                )
                `shouldBe` Right (BSC.pack "hello")

        it "decodes a hand-crafted RLE frame" $
            decompress
                (BS.pack [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0x0A, 0x53, 0, 0, 0x41])
                `shouldBe` Right (BS.replicate 10 0x41)

        it "consumes multi-segment headers and checksums" $
            decompress
                ( BS.pack
                    [ 0x28, 0xB5, 0x2F, 0xFD
                    , 0x10, 0x00
                    , 0x09, 0, 0, 0x78
                    , 1, 2, 3, 4
                    ]
                )
                `shouldBe` Right (BS.singleton 0x78)

        it "accepts every supported dictionary-id and content-size width" $
            mapM_
                (\descriptor -> decompress (headerForm descriptor) `shouldBe` Right BS.empty)
                [0x21, 0x62, 0xA3]

    describe "malformed frames" $ do
        it "rejects truncated, reserved, and trailing frame data" $
            mapM_
                (\frame -> decompress frame `shouldSatisfy` isLeft)
                [ BS.empty
                , BS.pack [0x28, 0xB5, 0x2F]
                , BSC.pack "not zstd data"
                , BS.pack [0x28, 0xB5, 0x2F, 0xFD, 0x2C]
                , BS.pack [0x28, 0xB5, 0x2F, 0xFD, 0x20]
                , BS.pack [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x29, 0, 0, 0x68]
                , BS.pack [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x53, 0, 0]
                , BS.pack [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x07, 0, 0]
                , BS.pack [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x01, 0, 0, 0xFF]
                , BS.pack [0x28, 0xB5, 0x2F, 0xFD, 0x20, 0, 0x05, 0, 0]
                ]

        it "rejects a truncated compressed bitstream" $ do
            let input = BSC.pack (concat (replicate 25 "abcabcabcabc"))
            compressed <- expectRight (compress input)
            blockTypes compressed `shouldBe` [2]
            decompress (BS.init compressed) `shouldSatisfy` isLeft

roundTrip :: BS.ByteString -> Expectation
roundTrip input = (compress input >>= decompress) `shouldBe` Right input

expectRight :: (Show failure) => Either failure value -> IO value
expectRight result =
    case result of
        Left failure -> expectationFailure (show failure) >> fail "unreachable"
        Right value -> pure value

isLeft :: Either failure value -> Bool
isLeft (Left _) = True
isLeft (Right _) = False

blockTypes :: BS.ByteString -> [Int]
blockTypes frame = go 13 []
  where
    go position result =
        let header =
                byteAt position
                    .|. (byteAt (position + 1) `shiftL` 8)
                    .|. (byteAt (position + 2) `shiftL` 16)
            isLast = header .&. 1 /= 0
            blockType = (header `shiftR` 1) .&. 3
            blockSize = header `shiftR` 3
            nextPosition = position + 3 + if blockType == 1 then 1 else blockSize
            nextResult = result ++ [blockType]
         in if isLast then nextResult else go nextPosition nextResult
    byteAt index = fromIntegral (BS.index frame index)

headerForm :: Word32 -> BS.ByteString
headerForm descriptor =
    BS.pack
        ( [0x28, 0xB5, 0x2F, 0xFD, fromIntegral descriptor]
            ++ replicate (dictionaryBytes + contentBytes) 0
            ++ [1, 0, 0]
        )
  where
    dictionaryBytes = case descriptor .&. 3 of 1 -> 1; 2 -> 2; _ -> 4
    contentBytes = case descriptor `shiftR` 6 of 0 -> 1; 1 -> 2; 2 -> 4; _ -> 8

pseudoRandomBytes :: Int -> BS.ByteString
pseudoRandomBytes count = BS.pack (take count (map fromIntegral (drop 1 seeds)))
  where
    seeds = iterate next (42 :: Word32)
    next seed = seed * 1664525 + 1013904223

hexBytes :: String -> BS.ByteString
hexBytes [] = BS.empty
hexBytes (high : low : rest) =
    BS.cons (fromIntegral (digitToInt high * 16 + digitToInt low)) (hexBytes rest)
hexBytes _ = error "hex fixture must contain complete bytes"
