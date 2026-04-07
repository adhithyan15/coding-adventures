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

    it "allows self loops if configured" $ do
      let g = DG.addEdge "A" "A" DG.newAllowSelfLoops
      DG.hasEdge "A" "A" g `shouldBe` True

    it "removes nodes and edges correctly" $ do
      let g = DG.addEdge "C" "D" $ DG.addEdge "B" "C" $ DG.addEdge "A" "B" DG.new
      let Right g2 = DG.removeNode "C" g
      DG.hasNode "C" g2 `shouldBe` False
      DG.hasEdge "B" "C" g2 `shouldBe` False
      DG.hasEdge "C" "D" g2 `shouldBe` False
      let Right g3 = DG.removeEdge "A" "B" g
      DG.hasEdge "A" "B" g3 `shouldBe` False

    it "returns nodes and edges properly" $ do
      let g = DG.addEdge "B" "C" $ DG.addEdge "A" "B" DG.new
      DG.nodes g `shouldBe` ["A", "B", "C"]
      DG.edges g `shouldBe` [("A", "B"), ("B", "C")]

    it "computes affected nodes securely" $ do
      let g = DG.addEdge "C" "D" $ DG.addEdge "B" "C" $ DG.addEdge "A" "B" DG.new
      DG.affectedNodes (Set.singleton "B") g `shouldBe` Set.fromList ["B", "C", "D"]

    it "handles failure scenarios" $ do
      let g = DG.new
      DG.removeNode "A" g `shouldBe` Left "Node not found: A"
      DG.removeEdge "A" "B" g `shouldBe` Left "Edge not found: A -> B"
      DG.predecessors "B" g `shouldBe` Left "Node not found: B"
      DG.successors "A" g `shouldBe` Left "Node not found: A"
      DG.transitiveClosure "A" g `shouldBe` Left "Node not found: A"

    it "evaluates isolated paths for functions" $ do
      let gCycle = DG.addEdge "C" "A" $ DG.addEdge "B" "C" $ DG.addEdge "A" "B" DG.new
      DG.independentGroups gCycle `shouldBe` Left "Cycle detected"
      let gEmpty = DG.new
      DG.independentGroups gEmpty `shouldBe` Right []
      DG.nodes gEmpty `shouldBe` []
      DG.edges gEmpty `shouldBe` []
      DG.hasNode "A" gEmpty `shouldBe` False
      DG.hasEdge "A" "B" gEmpty `shouldBe` False
      let gEdge = DG.addEdge "A" "B" gEmpty
      DG.hasEdge "B" "C" gEdge `shouldBe` False
      DG.removeEdge "A" "C" gEdge `shouldBe` Left "Edge not found: A -> C"
      gEmpty `shouldBe` DG.new
      show gEmpty `shouldNotBe` ""
      DG.size gEmpty `shouldBe` 0
