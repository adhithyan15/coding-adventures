module DerTlvSpec (spec) where

import CodingAdventures.DerTlv
import qualified Data.ByteString as BS
import Test.Hspec

spec :: Spec
spec = describe "native API" $ do
  it "exposes exact slices and defaults" $ do
    let result = decodeExact defaultDerLimits (BS.pack [4, 1, 42])
    fmap elementHeader result `shouldBe` Right (BS.pack [4, 1])
    fmap elementValue result `shouldBe` Right (BS.pack [42])
    maxElements defaultDerLimits `shouldBe` 4096
  it "renders stable error identifiers" $
    errorId TruncatedValue `shouldBe` "truncated-value"
  it "rejects cursor input limits before decoding" $
    newCursor defaultDerLimits {maxInputLength = 1} (BS.pack [5, 0])
      `shouldBe` Left (DerError InputLimitExceeded 0)
  it "rejects header plus value native-width overflow" $ do
    let limits = defaultDerLimits {maxInputLength = maxBound, maxValueLength = maxBound}
        encoded = BS.pack [4, 136, 127, 255, 255, 255, 255, 255, 255, 255]
    decodeOne limits encoded `shouldBe` Left (DerError LengthHostOverflow 1)
  it "keeps public diagnostics payload-blind" $ do
    let rendered = show (DerError LengthTooWide 1)
    rendered `shouldContain` "LengthTooWide"
    rendered `shouldContain` "1"
    rendered `shouldNotContain` "deadbeef"
