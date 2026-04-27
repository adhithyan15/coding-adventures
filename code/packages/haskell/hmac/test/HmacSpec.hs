module HmacSpec (spec) where

import Data.Char (ord)
import Data.Word (Word8)
import Hmac
import Test.Hspec

spec :: Spec
spec = describe "Hmac" $ do
    it "computes the RFC vector for HMAC-SHA1" $ do
        hmacSha1Hex keyHiThere (ascii "Hi There")
            `shouldBe` "b617318655057264e28bc0b6fb378c8ef146be00"

    it "computes the RFC vector for HMAC-SHA256" $ do
        hmacSha256Hex keyHiThere (ascii "Hi There")
            `shouldBe` "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"

    it "computes the RFC vector for HMAC-SHA512" $ do
        hmacSha512Hex keyHiThere (ascii "Hi There")
            `shouldBe` "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"

    it "verifies equal tags" $ do
        verifyTag [1, 2, 3, 4] [1, 2, 3, 4] `shouldBe` True

    it "rejects unequal tags" $ do
        verifyTag [1, 2, 3, 4] [1, 2, 3, 5] `shouldBe` False

keyHiThere :: [Word8]
keyHiThere = replicate 20 0x0b

ascii :: String -> [Word8]
ascii = map (fromIntegral . ord)
