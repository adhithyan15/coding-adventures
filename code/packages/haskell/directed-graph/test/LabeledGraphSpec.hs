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

    it "provides nodes, edges, labels and sizes" $ do
      let g = LG.addEdge "A" "B" "compile" LG.newLabeled
      LG.size g `shouldBe` 2
      LG.nodes g `shouldBe` ["A", "B"]
      LG.edges g `shouldBe` [("A", "B", "compile")]
      LG.labels "A" "B" g `shouldBe` Set.fromList ["compile"]

    it "queries predecessors by label" $ do
      let g = LG.addEdge "C" "A" "compile" $ LG.addEdge "B" "A" "test" LG.newLabeled
      LG.predecessorsWithLabel "A" "test" g `shouldBe` Right ["B"]

    it "removes nodes and validates fail paths" $ do
      let g = LG.addEdge "B" "C" "compile" $ LG.addEdge "A" "B" "test" LG.newLabeled
      let Right g2 = LG.removeNode "B" g
      LG.hasNode "B" g2 `shouldBe` False
      LG.hasEdge "A" "B" g2 `shouldBe` False
      LG.hasEdge "B" "C" g2 `shouldBe` False
      LG.removeNode "X" g `shouldBe` Left "Node not found: X"
      LG.removeEdge "X" "Y" "test" g `shouldBe` Left "Edge not found: X -> Y"
      LG.removeEdge "A" "B" "wrong" g `shouldBe` Left "Label not found: wrong"

    it "delegates top-sort, cycles and closures" $ do
      let g = LG.addEdge "A" "B" "compile" LG.newLabeled
      LG.hasCycle g `shouldBe` False
      LG.topologicalSort g `shouldBe` Right ["A", "B"]
      LG.transitiveClosure "A" g `shouldBe` Right (Set.fromList ["A", "B"])

    it "supports self-loops securely" $ do
      let g = LG.addEdge "A" "A" "self" LG.newLabeledAllowSelfLoops
      LG.hasEdge "A" "A" g `shouldBe` True
      LG.hasCycle g `shouldBe` True

    it "supports basic node addition and unlabeled neighbor checks" $ do
      let g = LG.addNode "A" LG.newLabeled
      LG.hasNode "A" g `shouldBe` True
      LG.hasNode "Z" g `shouldBe` False
      LG.predecessors "A" g `shouldBe` Right []
      LG.successors "A" g `shouldBe` Right []
      let g2 = LG.addEdge "A" "B" "test" g
      LG.predecessors "B" g2 `shouldBe` Right ["A"]
      LG.successors "A" g2 `shouldBe` Right ["B"]

    it "evaluates to left on invalid nodes for neighbors" $ do
      LG.predecessors "Z" LG.newLabeled `shouldBe` Left "Node not found: Z"
      LG.successors "Z" LG.newLabeled `shouldBe` Left "Node not found: Z"
      LG.predecessorsWithLabel "Z" "foo" LG.newLabeled `shouldBe` Left "Node not found: Z"
      LG.successorsWithLabel "Z" "foo" LG.newLabeled `shouldBe` Left "Node not found: Z"

    it "evaluates show and eq" $ do
      let g = LG.newLabeled
      g `shouldBe` LG.newLabeled
      show g `shouldNotBe` ""
