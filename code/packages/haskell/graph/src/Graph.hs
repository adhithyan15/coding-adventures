module Graph
    ( Graph
    , GraphPropertyBag
    , GraphPropertyValue(..)
    , WeightedEdge(..)
    , addEdge
    , addEdgeWithProperties
    , addNode
    , addNodeWithProperties
    , edgeWeight
    , edgeProperties
    , edges
    , empty
    , graphProperties
    , hasEdge
    , hasNode
    , neighbors
    , nodeProperties
    , nodes
    , removeEdge
    , removeEdgeProperty
    , removeGraphProperty
    , removeNode
    , removeNodeProperty
    , setEdgeProperty
    , setGraphProperty
    , setNodeProperty
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

data GraphPropertyValue
    = GraphString String
    | GraphNumber Double
    | GraphBool Bool
    | GraphNull
    deriving (Eq, Ord, Show)

type GraphPropertyBag = Map String GraphPropertyValue

data Graph = Graph
    { adjacency :: Map String (Map String Double)
    , graphPropertiesMap :: GraphPropertyBag
    , nodePropertiesMap :: Map String GraphPropertyBag
    , edgePropertiesMap :: Map (String, String) GraphPropertyBag
    }
    deriving (Eq, Show)

empty :: Graph
empty = Graph Map.empty Map.empty Map.empty Map.empty

addNode :: String -> Graph -> Graph
addNode node = addNodeWithProperties node Map.empty

addNodeWithProperties :: String -> GraphPropertyBag -> Graph -> Graph
addNodeWithProperties node properties graph =
    graph
        { adjacency = Map.insertWith (\_ old -> old) node Map.empty (adjacency graph)
        , nodePropertiesMap =
            Map.insertWith Map.union node properties (nodePropertiesMap graph)
        }

removeNode :: String -> Graph -> Either String Graph
removeNode node graph =
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
                cleanedEdges =
                    foldl
                        (\acc neighbor -> Map.delete (edgeKey node neighbor) acc)
                        (edgePropertiesMap graph)
                        (Map.keys neighborsMap)
             in Right
                    graph
                        { adjacency = Map.delete node cleaned
                        , nodePropertiesMap = Map.delete node (nodePropertiesMap graph)
                        , edgePropertiesMap = cleanedEdges
                        }
  where
    adjacencyMap = adjacency graph

hasNode :: String -> Graph -> Bool
hasNode node graph = Map.member node (adjacency graph)

nodes :: Graph -> [String]
nodes graph = Map.keys (adjacency graph)

size :: Graph -> Int
size graph = Map.size (adjacency graph)

addEdge :: String -> String -> Double -> Graph -> Graph
addEdge left right weight = addEdgeWithProperties left right weight Map.empty

addEdgeWithProperties :: String -> String -> Double -> GraphPropertyBag -> Graph -> Graph
addEdgeWithProperties left right weight properties graph =
    let normalizedWeight = if weight == 0 then 1 else weight
        prepared = addNode right (addNode left graph)
        adjacencyMap = adjacency prepared
        withLeft = Map.adjust (Map.insert right normalizedWeight) left adjacencyMap
        withRight = Map.adjust (Map.insert left normalizedWeight) right withLeft
        edgePropertyBag =
            Map.insert
                "weight"
                (GraphNumber normalizedWeight)
                properties
     in prepared
            { adjacency = withRight
            , edgePropertiesMap =
                Map.insertWith Map.union (edgeKey left right) edgePropertyBag (edgePropertiesMap prepared)
            }

removeEdge :: String -> String -> Graph -> Either String Graph
removeEdge left right graph =
    case Map.lookup left adjacencyMap >>= Map.lookup right of
        Nothing -> Left ("edge not found: " ++ left ++ " -- " ++ right)
        Just _ ->
            let withoutLeft = Map.adjust (Map.delete right) left adjacencyMap
                withoutRight = Map.adjust (Map.delete left) right withoutLeft
             in Right
                    graph
                        { adjacency = withoutRight
                        , edgePropertiesMap = Map.delete (edgeKey left right) (edgePropertiesMap graph)
                        }
  where
    adjacencyMap = adjacency graph

hasEdge :: String -> String -> Graph -> Bool
hasEdge left right graph =
    maybe False (Map.member right) (Map.lookup left (adjacency graph))

edgeWeight :: String -> String -> Graph -> Either String Double
edgeWeight left right graph =
    case Map.lookup left (adjacency graph) >>= Map.lookup right of
        Just weight -> Right weight
        Nothing -> Left ("edge not found: " ++ left ++ " -- " ++ right)

graphProperties :: Graph -> GraphPropertyBag
graphProperties = graphPropertiesMap

setGraphProperty :: String -> GraphPropertyValue -> Graph -> Graph
setGraphProperty key value graph =
    graph {graphPropertiesMap = Map.insert key value (graphPropertiesMap graph)}

removeGraphProperty :: String -> Graph -> Graph
removeGraphProperty key graph =
    graph {graphPropertiesMap = Map.delete key (graphPropertiesMap graph)}

nodeProperties :: String -> Graph -> Either String GraphPropertyBag
nodeProperties node graph
    | hasNode node graph = Right (Map.findWithDefault Map.empty node (nodePropertiesMap graph))
    | otherwise = Left ("node not found: " ++ node)

setNodeProperty :: String -> String -> GraphPropertyValue -> Graph -> Either String Graph
setNodeProperty node key value graph
    | hasNode node graph =
        Right
            graph
                { nodePropertiesMap =
                    Map.insertWith
                        Map.union
                        node
                        (Map.singleton key value)
                        (nodePropertiesMap graph)
                }
    | otherwise = Left ("node not found: " ++ node)

removeNodeProperty :: String -> String -> Graph -> Either String Graph
removeNodeProperty node key graph
    | hasNode node graph =
        Right
            graph
                { nodePropertiesMap =
                    Map.adjust (Map.delete key) node (nodePropertiesMap graph)
                }
    | otherwise = Left ("node not found: " ++ node)

edgeProperties :: String -> String -> Graph -> Either String GraphPropertyBag
edgeProperties left right graph =
    case edgeWeight left right graph of
        Left err -> Left err
        Right weight ->
            Right
                ( Map.insert
                    "weight"
                    (GraphNumber weight)
                    (Map.findWithDefault Map.empty (edgeKey left right) (edgePropertiesMap graph))
                )

setEdgeProperty :: String -> String -> String -> GraphPropertyValue -> Graph -> Either String Graph
setEdgeProperty left right key value graph
    | not (hasEdge left right graph) = Left ("edge not found: " ++ left ++ " -- " ++ right)
    | key == "weight" =
        case value of
            GraphNumber weight ->
                Right
                    (setEdgePropertyBagValue left right key value (setEdgeWeight left right weight graph))
            _ -> Left "edge property 'weight' must be numeric"
    | otherwise = Right (setEdgePropertyBagValue left right key value graph)

removeEdgeProperty :: String -> String -> String -> Graph -> Either String Graph
removeEdgeProperty left right key graph
    | not (hasEdge left right graph) = Left ("edge not found: " ++ left ++ " -- " ++ right)
    | key == "weight" =
        Right
            ( setEdgePropertyBagValue
                left
                right
                "weight"
                (GraphNumber 1)
                (setEdgeWeight left right 1 graph)
            )
    | otherwise =
        Right
            graph
                { edgePropertiesMap =
                    Map.adjust (Map.delete key) (edgeKey left right) (edgePropertiesMap graph)
                }

edges :: Graph -> [WeightedEdge]
edges graph =
    nub
        [ WeightedEdge first second weight
        | (left, neighborsMap) <- Map.toList adjacencyMap
        , (right, weight) <- Map.toList neighborsMap
        , let (first, second) = canonicalEndpoints left right
        ]
  where
    adjacencyMap = adjacency graph

neighbors :: String -> Graph -> Either String [String]
neighbors node graph =
    case Map.lookup node (adjacency graph) of
        Nothing -> Left ("node not found: " ++ node)
        Just neighborsMap -> Right (Map.keys neighborsMap)

canonicalEndpoints :: String -> String -> (String, String)
canonicalEndpoints left right
    | left <= right = (left, right)
    | otherwise = (right, left)

edgeKey :: String -> String -> (String, String)
edgeKey = canonicalEndpoints

setEdgeWeight :: String -> String -> Double -> Graph -> Graph
setEdgeWeight left right weight graph =
    graph
        { adjacency =
            Map.adjust (Map.insert right weight) left $
                Map.adjust (Map.insert left weight) right (adjacency graph)
        }

setEdgePropertyBagValue :: String -> String -> String -> GraphPropertyValue -> Graph -> Graph
setEdgePropertyBagValue left right key value graph =
    graph
        { edgePropertiesMap =
            Map.insertWith
                Map.union
                (edgeKey left right)
                (Map.singleton key value)
                (edgePropertiesMap graph)
        }
