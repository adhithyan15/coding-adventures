-- | Security regression tests: hostile headers must not drive work or memory.
--
-- Each case is a few bytes whose header claims an enormous count or length.
-- The Haskell port validates headers against the payload before iterating, so
-- hostile input must be rejected promptly (never traversed from the claim).
-- Also round-trips every input of length <= 2 and a seeded pseudo-random sample.
module HostileHeadersSpec (spec) where

import qualified Data.ByteString as BS
import Data.Either (isLeft)
import Data.Word (Word8)
import Lz78
import Test.Hspec

bytes :: [Int] -> BS.ByteString
bytes = BS.pack . map fromIntegral

lcg :: Int -> Int
lcg x = (x * 1103515245 + 12345) `mod` 2147483648

randomInputs :: [BS.ByteString]
randomInputs = go (300 :: Int) 0x5EED (0 :: Int)
  where
    go 0 _ _ = []
    go n s i =
        let s1 = lcg s
            len = 3 + (s1 `div` 65536) `mod` 597
            alpha = [2, 4, 16, 256] !! (i `mod` 4)
            (bs, s2) = gen len s1 alpha []
         in BS.pack bs : go (n - 1) s2 (i + 1)
    gen :: Int -> Int -> Int -> [Word8] -> ([Word8], Int)
    gen 0 s _ acc = (acc, s)
    gen k s alpha acc =
        let s' = lcg s
         in gen (k - 1) s' alpha (fromIntegral ((s' `div` 65536) `mod` alpha) : acc)

shortInputs :: [BS.ByteString]
shortInputs =
    BS.empty
        : [BS.pack [a] | a <- [0 .. 255]]
        ++ [BS.pack [a, b] | a <- [0 .. 255], b <- [0 .. 255]]

spec :: Spec
spec = describe "hostile headers" $ do
    it "rejects a max header with no tokens" $
        decompress (BS.replicate 8 255) `shouldSatisfy` isLeft

    it "rejects a count larger than the remaining bytes" $
        decompress (bytes [0, 0, 0, 2, 255, 255, 255, 255, 0, 0, 104, 0, 0, 0, 105, 0]) `shouldSatisfy` isLeft

    it "rejects a dangling dictionary index" $
        decompress (bytes [0, 0, 0, 10, 0, 0, 0, 2, 0, 0, 97, 0, 0x27, 0x0F, 98, 0]) `shouldSatisfy` isLeft

    it "stops chain tokens at the declared length" $ do
        let tokens = [Token (fromIntegral i) 97 | i <- [0 .. 19999 :: Int]]
        fmap BS.length (decode tokens (Just 1000)) `shouldBe` Right 1000

    it "round-trips every input of length <= 2" $
        all (\d -> decompress (compressDefault d) == Right d) shortInputs `shouldBe` True

    it "round-trips a seeded random sample" $
        all (\d -> decompress (compressDefault d) == Right d) randomInputs `shouldBe` True
