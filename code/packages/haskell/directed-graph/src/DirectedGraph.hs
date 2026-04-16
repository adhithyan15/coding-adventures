module DirectedGraph
    ( DirectedGraph
    , GraphError(..)
    , addEdge
    , addNode
    , empty
    , hasEdge
    , hasNode
    , independentGroups
    , nodes
    , predecessors
    , successors
    , transitiveDependents
    , transitivePredecessors
    ) where

import Data.List (sort)
import qualified Data.Map.Strict as Map
import Data.Map.Strict (Map)
import qualified Data.Set as Set
import Data.Set (Set)

data GraphError
    = CycleError
    | NodeNotFound String
    deriving (Eq, Show)

data DirectedGraph = DirectedGraph
    { graphNodes :: Set String
    , graphForward :: Map String (Set String)
    , graphReverse :: Map String (Set String)
    }
    deriving (Eq, Show)

empty :: DirectedGraph
empty = DirectedGraph Set.empty Map.empty Map.empty

addNode :: String -> DirectedGraph -> DirectedGraph
addNode node graph =
    graph
        { graphNodes = Set.insert node (graphNodes graph)
        , graphForward = Map.insertWith Set.union node Set.empty (graphForward graph)
        , graphReverse = Map.insertWith Set.union node Set.empty (graphReverse graph)
        }

addEdge :: String -> String -> DirectedGraph -> DirectedGraph
addEdge fromNode toNode =
    withNodes
  where
    withNodes graph =
        let graphWithNodes = addNode toNode (addNode fromNode graph)
         in graphWithNodes
                { graphForward = Map.insertWith Set.union fromNode (Set.singleton toNode) (graphForward graphWithNodes)
                , graphReverse = Map.insertWith Set.union toNode (Set.singleton fromNode) (graphReverse graphWithNodes)
                }

hasNode :: String -> DirectedGraph -> Bool
hasNode node graph = Set.member node (graphNodes graph)

hasEdge :: String -> String -> DirectedGraph -> Bool
hasEdge fromNode toNode graph =
    maybe False (Set.member toNode) (Map.lookup fromNode (graphForward graph))

nodes :: DirectedGraph -> [String]
nodes graph = sort (Set.toList (graphNodes graph))

successors :: String -> DirectedGraph -> Either GraphError [String]
successors node graph =
    case Map.lookup node (graphForward graph) of
        Nothing -> Left (NodeNotFound node)
        Just nodeSet -> Right (sort (Set.toList nodeSet))

predecessors :: String -> DirectedGraph -> Either GraphError [String]
predecessors node graph =
    case Map.lookup node (graphReverse graph) of
        Nothing -> Left (NodeNotFound node)
        Just nodeSet -> Right (sort (Set.toList nodeSet))

transitivePredecessors :: String -> DirectedGraph -> [String]
transitivePredecessors start graph = sort (walk Set.empty [start])
  where
    walk visited [] = Set.toList visited
    walk visited (current : rest) =
        let direct =
                Set.toList
                    (Map.findWithDefault Set.empty current (graphReverse graph))
            unseen = filter (`Set.notMember` visited) direct
         in walk (foldr Set.insert visited unseen) (unseen ++ rest)

transitiveDependents :: String -> DirectedGraph -> [String]
transitiveDependents start graph = sort (walk Set.empty [start])
  where
    walk visited [] = Set.toList visited
    walk visited (current : rest) =
        let direct =
                Set.toList
                    (Map.findWithDefault Set.empty current (graphForward graph))
            unseen = filter (`Set.notMember` visited) direct
         in walk (foldr Set.insert visited unseen) (unseen ++ rest)

independentGroups :: DirectedGraph -> Either GraphError [[String]]
independentGroups graph = loop initialInDegree Set.empty initialReady
  where
    initialInDegree =
        Map.fromList
            [ (node, Set.size (Map.findWithDefault Set.empty node (graphReverse graph)))
            | node <- Set.toList (graphNodes graph)
            ]
    initialReady =
        sort [node | (node, degree) <- Map.toList initialInDegree, degree == 0]

    loop _ processed []
        | Set.size processed == Set.size (graphNodes graph) = Right []
        | otherwise = Left CycleError
    loop inDegree processed ready =
        let updatedDegrees =
                foldl
                    (\degrees node ->
                        let nextNodes =
                                Set.toList
                                    (Map.findWithDefault Set.empty node (graphForward graph))
                         in foldl
                                (\inner successor ->
                                    Map.adjust (\value -> max 0 (value - 1)) successor inner
                                )
                                degrees
                                nextNodes
                    )
                    inDegree
                    ready
            processed' = foldr Set.insert processed ready
            nextReady =
                sort
                    [ node
                    | (node, degree) <- Map.toList updatedDegrees
                    , degree == 0
                    , node `Set.notMember` processed'
                    ]
         in fmap (ready :) (loop updatedDegrees processed' nextReady)
