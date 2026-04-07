module DirectedGraphSpec (spec) where

import Test.Hspec
import qualified DirectedGraph as DG
import qualified Data.Set as Set

spec :: Spec
spec = do
  describe "DirectedGraph Operations" $ do
    it "creates and checks nodes" $ do
      let g = DG.addNode "A" DG.new
      DG.hasNode "A" g `shouldBe` True
      DG.hasNode "B" g `shouldBe` False
      DG.size g `shouldBe` 1

    it "adds edges and correctly links them" $ do
      let g = DG.addEdge "A" "B" DG.new
      DG.hasEdge "A" "B" g `shouldBe` True
      DG.hasEdge "B" "A" g `shouldBe` False
      DG.successors "A" g `shouldBe` Right ["B"]
      DG.predecessors "B" g `shouldBe` Right ["A"]
      DG.size g `shouldBe` 2

    it "detects cycles" $ do
      let g = DG.addEdge "C" "A" (DG.addEdge "B" "C" (DG.addEdge "A" "B" DG.new))
      DG.hasCycle g `shouldBe` True
      DG.topologicalSort g `shouldBe` Left "Cycle detected"

    it "topologically sorts acyclic graphs" $ do
      -- A -> B, A -> C, B -> D, C -> D
      let g = DG.addEdge "C" "D" $ DG.addEdge "B" "D" $ DG.addEdge "A" "C" $ DG.addEdge "A" "B" DG.new
      DG.hasCycle g `shouldBe` False
      let Right sorted = DG.topologicalSort g
      sorted `shouldSatisfy` \s -> head s == "A" && last s == "D"

    it "computes independent groups securely" $ do
      let g = DG.addEdge "C" "D" $ DG.addEdge "B" "D" $ DG.addEdge "A" "C" $ DG.addEdge "A" "B" DG.new
      DG.independentGroups g `shouldBe` Right [["A"], ["B", "C"], ["D"]]

    it "computes transitive dependents correctly" $ do
      let g = DG.addEdge "C" "D" $ DG.addEdge "B" "D" $ DG.addEdge "A" "C" $ DG.addEdge "A" "B" DG.new
      DG.transitiveDependents "A" g `shouldBe` Right (Set.fromList ["A", "B", "C", "D"])
      DG.transitiveDependents "B" g `shouldBe` Right (Set.fromList ["B", "D"])
