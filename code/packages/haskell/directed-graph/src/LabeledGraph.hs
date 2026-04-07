module LabeledGraph
    ( LabeledGraph
    , newLabeled
    , newLabeledAllowSelfLoops
    , addNode
    , removeNode
    , hasNode
    , nodes
    , size
    , addEdge
    , removeEdge
    , hasEdge
    , hasEdgeWithLabel
    , edges
    , labels
    , predecessors
    , predecessorsWithLabel
    , successors
    , successorsWithLabel
    , topologicalSort
    , hasCycle
    , transitiveClosure
    , graph
    ) where

import qualified DirectedGraph as DG
import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import Data.Map.Strict (Map)
import Data.Set (Set)
import Data.List (sort)
import Data.Maybe (fromMaybe)

data LabeledGraph = LabeledGraph
    { graph :: DG.Graph 
    , edgeLabels :: Map (String, String) (Set String)
    } deriving (Show, Eq)

newLabeled :: LabeledGraph
newLabeled = LabeledGraph DG.new Map.empty

newLabeledAllowSelfLoops :: LabeledGraph
newLabeledAllowSelfLoops = LabeledGraph DG.newAllowSelfLoops Map.empty

addNode :: String -> LabeledGraph -> LabeledGraph
addNode n lg = lg { graph = DG.addNode n (graph lg) }

removeNode :: String -> LabeledGraph -> Either String LabeledGraph
removeNode n lg = 
    case DG.removeNode n (graph lg) of
        Left err -> Left err
        Right g' -> 
            let newLabels = Map.filterWithKey (\(f, t) _ -> f /= n && t /= n) (edgeLabels lg)
            in Right $ lg { graph = g', edgeLabels = newLabels }

hasNode :: String -> LabeledGraph -> Bool
hasNode n lg = DG.hasNode n (graph lg)

nodes :: LabeledGraph -> [String]
nodes lg = DG.nodes (graph lg)

size :: LabeledGraph -> Int
size lg = DG.size (graph lg)

addEdge :: String -> String -> String -> LabeledGraph -> LabeledGraph
addEdge f t l lg = 
    let g' = DG.addEdge f t (graph lg)
        key = (f, t)
        ls = fromMaybe Set.empty (Map.lookup key (edgeLabels lg))
        newLabels = Map.insert key (Set.insert l ls) (edgeLabels lg)
    in lg { graph = g', edgeLabels = newLabels }

removeEdge :: String -> String -> String -> LabeledGraph -> Either String LabeledGraph
removeEdge f t l lg = 
    let key = (f, t)
    in case Map.lookup key (edgeLabels lg) of
        Nothing -> Left $ "Edge not found: " ++ f ++ " -> " ++ t
        Just ls -> 
            if not (Set.member l ls)
                then Left $ "Label not found: " ++ l
                else 
                    let ls' = Set.delete l ls
                    in if Set.null ls' 
                        then case DG.removeEdge f t (graph lg) of
                            Left err -> Left err 
                            Right g' -> Right $ lg 
                                { graph = g'
                                , edgeLabels = Map.delete key (edgeLabels lg)
                                }
                        else Right $ lg { edgeLabels = Map.insert key ls' (edgeLabels lg) }

hasEdge :: String -> String -> LabeledGraph -> Bool
hasEdge f t lg = DG.hasEdge f t (graph lg)

hasEdgeWithLabel :: String -> String -> String -> LabeledGraph -> Bool
hasEdgeWithLabel f t l lg = 
    case Map.lookup (f, t) (edgeLabels lg) of
        Nothing -> False
        Just ls -> Set.member l ls

edges :: LabeledGraph -> [(String, String, String)]
edges lg = 
    let es = [ (f, t, l) | ((f, t), ls) <- Map.toList (edgeLabels lg), l <- Set.toList ls ]
    in sort es

labels :: String -> String -> LabeledGraph -> Set String
labels f t lg = fromMaybe Set.empty (Map.lookup (f, t) (edgeLabels lg))

predecessors :: String -> LabeledGraph -> Either String [String]
predecessors n lg = DG.predecessors n (graph lg)

predecessorsWithLabel :: String -> String -> LabeledGraph -> Either String [String]
predecessorsWithLabel n l lg = 
    case DG.predecessors n (graph lg) of
        Left err -> Left err
        Right preds -> 
            Right $ filter (\p -> hasEdgeWithLabel p n l lg) preds

successors :: String -> LabeledGraph -> Either String [String]
successors n lg = DG.successors n (graph lg)

successorsWithLabel :: String -> String -> LabeledGraph -> Either String [String]
successorsWithLabel n l lg = 
    case DG.successors n (graph lg) of
        Left err -> Left err
        Right succs -> 
            Right $ filter (\s -> hasEdgeWithLabel n s l lg) succs

topologicalSort :: LabeledGraph -> Either String [String]
topologicalSort lg = DG.topologicalSort (graph lg)

hasCycle :: LabeledGraph -> Bool
hasCycle lg = DG.hasCycle (graph lg)

transitiveClosure :: String -> LabeledGraph -> Either String (Set String)
transitiveClosure n lg = DG.transitiveClosure n (graph lg)
