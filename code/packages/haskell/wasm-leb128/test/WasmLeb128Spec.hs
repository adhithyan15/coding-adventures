module WasmLeb128Spec (spec) where

import qualified Data.ByteString as BS
import Test.Hspec
import WasmLeb128

spec :: Spec
spec = describe "WasmLeb128" $ do
    it "decodes unsigned values" $ do
        decodeUnsigned (BS.pack [0xE5, 0x8E, 0x26]) 0 `shouldBe` Right (624485, 3)

    it "decodes signed negative values" $ do
        decodeSigned (BS.pack [0x7E]) 0 `shouldBe` Right (-2, 1)
        decodeSigned (BS.pack [0x40]) 0 `shouldBe` Right (-64, 1)

    it "round-trips signed and unsigned encodings" $ do
        let unsignedValues = [0, 1, 127, 128, 624485, 4294967295]
            signedValues = [-2147483648, -129, -128, -65, -1, 0, 1, 64, 2147483647]
        map (\value -> fmap fst (decodeUnsigned (encodeUnsigned value) 0)) unsignedValues
            `shouldBe` map Right unsignedValues
        map (\value -> fmap fst (decodeSigned (encodeSigned value) 0)) signedValues
            `shouldBe` map Right signedValues

    it "reports unterminated values" $ do
        decodeUnsigned (BS.pack [0x80, 0x80]) 0 `shouldSatisfy` isLeft
        decodeSigned BS.empty 0 `shouldSatisfy` isLeft

isLeft :: Either a b -> Bool
isLeft eitherValue =
    case eitherValue of
        Left _ -> True
        Right _ -> False
