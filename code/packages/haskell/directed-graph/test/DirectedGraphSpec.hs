module DirectedGraphSpec (spec) where

import Test.Hspec

import DirectedGraph

spec :: Spec
spec = do
    describe "DirectedGraph" $ do
        it "adds directed edges and queries predecessors and successors" $ do
            let graph = addEdge "A" "B" (addEdge "B" "C" empty)
            hasEdge "A" "B" graph `shouldBe` True
            hasEdge "B" "A" graph `shouldBe` False
            successors "B" graph `shouldBe` Right ["C"]
            predecessors "B" graph `shouldBe` Right ["A"]

        it "computes transitive predecessors and dependents" $ do
            let graph = addEdge "A" "B" (addEdge "B" "C" empty)
            transitivePredecessors "C" graph `shouldBe` ["A", "B"]
            transitiveDependents "A" graph `shouldBe` ["B", "C"]

        it "groups nodes by dependency level" $ do
            let graph = addEdge "A" "C" (addEdge "B" "C" empty)
            independentGroups graph `shouldBe` Right [["A", "B"], ["C"]]
