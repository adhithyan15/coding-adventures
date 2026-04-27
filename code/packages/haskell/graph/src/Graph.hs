module Graph
    ( Graph
    , WeightedEdge(..)
    , addEdge
    , addNode
    , edgeWeight
    , edges
    , empty
    , hasEdge
    , hasNode
    , neighbors
    , nodes
    , removeEdge
    , removeNode
    , size
    ) where

import Data.List (nub)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)

data WeightedEdge = WeightedEdge
    { edgeLeft :: String
    , edgeRight :: String
    , edgeWeightValue :: Double
    }
    deriving (Eq, Ord, Show)

newtype Graph = Graph
    { adjacency :: Map String (Map String Double)
    }
    deriving (Eq, Show)

empty :: Graph
empty = Graph Map.empty

addNode :: String -> Graph -> Graph
addNode node (Graph adjacencyMap) =
    Graph (Map.insertWith (\_ old -> old) node Map.empty adjacencyMap)

removeNode :: String -> Graph -> Either String Graph
removeNode node (Graph adjacencyMap) =
    case Map.lookup node adjacencyMap of
        Nothing -> Left ("node not found: " ++ node)
        Just neighborsMap ->
            let cleaned =
                    foldl
                        (\acc neighbor ->
                            Map.adjust (Map.delete node) neighbor acc
                        )
                        adjacencyMap
                        (Map.keys neighborsMap)
             in Right (Graph (Map.delete node cleaned))

hasNode :: String -> Graph -> Bool
hasNode node (Graph adjacencyMap) = Map.member node adjacencyMap

nodes :: Graph -> [String]
nodes (Graph adjacencyMap) = Map.keys adjacencyMap

size :: Graph -> Int
size (Graph adjacencyMap) = Map.size adjacencyMap

addEdge :: String -> String -> Double -> Graph -> Graph
addEdge left right weight graph =
    let normalizedWeight = if weight == 0 then 1 else weight
        Graph adjacencyMap = addNode right (addNode left graph)
        withLeft = Map.adjust (Map.insert right normalizedWeight) left adjacencyMap
        withRight = Map.adjust (Map.insert left normalizedWeight) right withLeft
     in Graph withRight

removeEdge :: String -> String -> Graph -> Either String Graph
removeEdge left right (Graph adjacencyMap) =
    case Map.lookup left adjacencyMap >>= Map.lookup right of
        Nothing -> Left ("edge not found: " ++ left ++ " -- " ++ right)
        Just _ ->
            let withoutLeft = Map.adjust (Map.delete right) left adjacencyMap
                withoutRight = Map.adjust (Map.delete left) right withoutLeft
             in Right (Graph withoutRight)

hasEdge :: String -> String -> Graph -> Bool
hasEdge left right (Graph adjacencyMap) =
    maybe False (Map.member right) (Map.lookup left adjacencyMap)

edgeWeight :: String -> String -> Graph -> Either String Double
edgeWeight left right (Graph adjacencyMap) =
    case Map.lookup left adjacencyMap >>= Map.lookup right of
        Just weight -> Right weight
        Nothing -> Left ("edge not found: " ++ left ++ " -- " ++ right)

edges :: Graph -> [WeightedEdge]
edges (Graph adjacencyMap) =
    nub
        [ WeightedEdge first second weight
        | (left, neighborsMap) <- Map.toList adjacencyMap
        , (right, weight) <- Map.toList neighborsMap
        , let (first, second) = canonicalEndpoints left right
        ]

neighbors :: String -> Graph -> Either String [String]
neighbors node (Graph adjacencyMap) =
    case Map.lookup node adjacencyMap of
        Nothing -> Left ("node not found: " ++ node)
        Just neighborsMap -> Right (Map.keys neighborsMap)

canonicalEndpoints :: String -> String -> (String, String)
canonicalEndpoints left right
    | left <= right = (left, right)
    | otherwise = (right, left)
