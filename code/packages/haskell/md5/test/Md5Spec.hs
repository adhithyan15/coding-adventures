module Md5Spec (spec) where

import Data.Char (ord)
import Data.Word (Word8)
import Md5
import Test.Hspec

spec :: Spec
spec = describe "Md5" $ do
    it "hashes the empty string" $ do
        hexString [] `shouldBe` "d41d8cd98f00b204e9800998ecf8427e"

    it "hashes abc" $ do
        hexString (ascii "abc") `shouldBe` "900150983cd24fb0d6963f7d28e17f72"

    it "hashes message digest" $ do
        hexString (ascii "message digest") `shouldBe` "f96b697d7cb7938d525a2f31aaf161d0"

    it "returns a 16-byte digest" $ do
        length (sumMd5 (ascii "coding adventures")) `shouldBe` 16

ascii :: String -> [Word8]
ascii = map (fromIntegral . ord)
