module GraphSpec (spec) where

import Test.Hspec

import Graph
import qualified Data.Map.Strict as Map

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

        it "tracks graph node and edge property bags" $ do
            let graph0 =
                    setGraphProperty "name" (GraphString "city-map") $
                        setGraphProperty "version" (GraphNumber 1) empty
            graphProperties graph0 `shouldBe` Map.fromList [("name", GraphString "city-map"), ("version", GraphNumber 1)]

            let graph1 = removeGraphProperty "version" graph0
            graphProperties graph1 `shouldBe` Map.fromList [("name", GraphString "city-map")]

            let graph2 =
                    addNodeWithProperties "A" (Map.fromList [("kind", GraphString "input")]) graph1
            let graph3 =
                    addNodeWithProperties "A" (Map.fromList [("trainable", GraphBool False)]) graph2
            let Right graph4 = setNodeProperty "A" "slot" (GraphNumber 0) graph3
            nodeProperties "A" graph4
                `shouldBe` Right
                    ( Map.fromList
                        [ ("kind", GraphString "input")
                        , ("trainable", GraphBool False)
                        , ("slot", GraphNumber 0)
                        ]
                    )

            let Right graph5 = removeNodeProperty "A" "slot" graph4
            nodeProperties "A" graph5
                `shouldBe` Right
                    (Map.fromList [("kind", GraphString "input"), ("trainable", GraphBool False)])

            let graph6 =
                    addEdgeWithProperties
                        "A"
                        "B"
                        2.5
                        (Map.fromList [("role", GraphString "distance")])
                        graph5
            edgeProperties "B" "A" graph6
                `shouldBe` Right
                    (Map.fromList [("role", GraphString "distance"), ("weight", GraphNumber 2.5)])

            let Right graph7 = setEdgeProperty "B" "A" "weight" (GraphNumber 7) graph6
            edgeWeight "A" "B" graph7 `shouldBe` Right 7
            let Right graph8 = setEdgeProperty "A" "B" "trainable" (GraphBool True) graph7
            let Right graph9 = removeEdgeProperty "A" "B" "role" graph8
            edgeProperties "A" "B" graph9
                `shouldBe` Right
                    (Map.fromList [("weight", GraphNumber 7), ("trainable", GraphBool True)])
