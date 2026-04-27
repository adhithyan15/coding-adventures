module Sha256Spec (spec) where

import Data.Char (ord)
import Data.Word (Word8)
import Sha256
import Test.Hspec

spec :: Spec
spec = describe "Sha256" $ do
    it "hashes abc to the FIPS vector" $ do
        sha256Hex (ascii "abc")
            `shouldBe` "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"

    it "hashes the empty string" $ do
        sha256Hex []
            `shouldBe` "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

    it "returns a 32-byte digest" $ do
        length (sha256 (ascii "coding adventures")) `shouldBe` 32

ascii :: String -> [Word8]
ascii = map (fromIntegral . ord)
