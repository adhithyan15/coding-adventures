module HashFunctionsSpec (spec) where

import qualified Data.ByteString.Char8 as BC
import HashFunctions
import Test.Hspec

spec :: Spec
spec = describe "HashFunctions" $ do
    it "exposes a non-empty description" $ do
        description `shouldSatisfy` (not . null)

    it "hashes bytes deterministically" $ do
        hashBytes64 (BC.pack "mini-redis")
            `shouldBe` hashBytes64 (BC.pack "mini-redis")

    it "changes when the input changes" $ do
        hashBytes64 (BC.pack "alpha")
            `shouldNotBe` hashBytes64 (BC.pack "beta")
