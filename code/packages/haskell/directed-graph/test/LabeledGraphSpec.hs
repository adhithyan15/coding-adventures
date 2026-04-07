module LabeledGraphSpec (spec) where

import Test.Hspec
import qualified LabeledGraph as LG
import qualified Data.Set as Set

spec :: Spec
spec = do
  describe "LabeledGraph Operations" $ do
    it "adds edges with labels safely" $ do
      let g = LG.addEdge "A" "B" "compile" LG.newLabeled
      LG.hasEdge "A" "B" g `shouldBe` True
      LG.hasEdgeWithLabel "A" "B" "compile" g `shouldBe` True
      LG.hasEdgeWithLabel "A" "B" "test" g `shouldBe` False
      
    it "removes edge if last label is removed" $ do
      let g = LG.addEdge "A" "B" "compile" LG.newLabeled
      let Right g2 = LG.removeEdge "A" "B" "compile" g
      LG.hasEdge "A" "B" g2 `shouldBe` False

    it "keeps edge if other labels exist" $ do
      let g = LG.addEdge "A" "B" "compile" $ LG.addEdge "A" "B" "test" LG.newLabeled
      let Right g2 = LG.removeEdge "A" "B" "compile" g
      LG.hasEdge "A" "B" g2 `shouldBe` True
      LG.hasEdgeWithLabel "A" "B" "test" g2 `shouldBe` True

    it "queries successors by label" $ do
      let g = LG.addEdge "A" "C" "compile" $ LG.addEdge "A" "B" "test" LG.newLabeled
      LG.successorsWithLabel "A" "test" g `shouldBe` Right ["B"]
      LG.successorsWithLabel "A" "compile" g `shouldBe` Right ["C"]
