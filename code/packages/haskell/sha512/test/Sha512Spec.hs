module Sha512Spec (spec) where

import Data.Char (ord)
import Data.Word (Word8)
import Sha512
import Test.Hspec

spec :: Spec
spec = describe "Sha512" $ do
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
