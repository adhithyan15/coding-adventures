module Sha1Spec (spec) where

import Data.Char (ord)
import Data.Word (Word8)
import Sha1
import Test.Hspec

spec :: Spec
spec = describe "Sha1" $ do
    it "hashes abc to the FIPS vector" $ do
        sha1Hex (ascii "abc")
            `shouldBe` "a9993e364706816aba3e25717850c26c9cd0d89d"

    it "hashes the empty string" $ do
        sha1Hex []
            `shouldBe` "da39a3ee5e6b4b0d3255bfef95601890afd80709"

    it "returns a 20-byte digest" $ do
        length (sha1 (ascii "coding adventures")) `shouldBe` 20

ascii :: String -> [Word8]
ascii = map (fromIntegral . ord)
