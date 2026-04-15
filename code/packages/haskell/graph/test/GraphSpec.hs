module GraphSpec (spec) where

import Test.Hspec

import Graph

spec :: Spec
spec = do
    describe "Graph" $ do
        it "adds nodes and edges symmetrically" $ do
            let graph = addEdge "A" "B" 2.5 empty
            hasNode "A" graph `shouldBe` True
            hasEdge "A" "B" graph `shouldBe` True
            hasEdge "B" "A" graph `shouldBe` True
            edgeWeight "A" "B" graph `shouldBe` Right 2.5

        it "lists neighbors in sorted order" $ do
            let graph = addEdge "A" "C" 1 (addEdge "A" "B" 1 empty)
            neighbors "A" graph `shouldBe` Right ["B", "C"]

        it "removes edges and nodes" $ do
            let graph = addEdge "A" "B" 1 (addEdge "B" "C" 1 empty)
            fmap (hasEdge "A" "B") (removeEdge "A" "B" graph) `shouldBe` Right False
            fmap (hasNode "B") (removeNode "B" graph) `shouldBe` Right False
