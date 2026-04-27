module RadixTreeSpec (spec) where

import Prelude hiding (lookup)

import qualified Data.ByteString.Char8 as BC
import RadixTree
import Test.Hspec

spec :: Spec
spec = describe "RadixTree" $ do
    it "stores keys and performs prefix lookups" $ do
        let valuesTree =
                insert (BC.pack "foo_b")
                    ()
                    (insert (BC.pack "foo_a") () (insert (BC.pack "bar") () empty))
        contains (BC.pack "bar") valuesTree `shouldBe` True
        lookup (BC.pack "foo_a") valuesTree `shouldBe` Just ()
        keysWithPrefix (BC.pack "foo") valuesTree
            `shouldBe` map BC.pack ["foo_a", "foo_b"]
        keys (delete (BC.pack "bar") valuesTree)
            `shouldBe` map BC.pack ["foo_a", "foo_b"]

    it "exposes a non-empty description" $ do
        description `shouldSatisfy` (not . null)
