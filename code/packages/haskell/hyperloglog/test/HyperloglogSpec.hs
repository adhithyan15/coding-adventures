module HyperloglogSpec (spec) where

import qualified Data.ByteString.Char8 as BC
import Hyperloglog
import Test.Hspec

spec :: Spec
spec = describe "Hyperloglog" $ do
    it "tracks approximate small-cardinality counts" $ do
        let (valuesHll, changed) =
                addMany (map BC.pack ["a", "b", "c"]) new
        changed `shouldBe` True
        count valuesHll `shouldSatisfy` (\value -> value >= 3 && value <= 4)

    it "merges sketches" $ do
        let leftHll = fst (addMany (map BC.pack ["a", "b", "c"]) new)
            rightHll = fst (addMany (map BC.pack ["b", "c", "d"]) new)
        count (merge leftHll rightHll)
            `shouldSatisfy` (\value -> value >= 4 && value <= 5)

    it "exposes a non-empty description" $ do
        description `shouldSatisfy` (not . null)
