module DirectedGraph
    ( Graph
    , new
    , newAllowSelfLoops
    , addNode
    , addEdge
    , removeNode
    , removeEdge
    , hasNode
    , hasEdge
    , nodes
    , edges
    , predecessors
    , successors
    , size
    , topologicalSort
    , hasCycle
    , transitiveClosure
    , transitiveDependents
    , independentGroups
    , affectedNodes
    ) where

import qualified Data.Map.Strict as Map
import qualified Data.Set as Set
import Data.Map.Strict (Map)
import Data.Set (Set)
import Data.List (sort)
import Data.Maybe (fromMaybe)
import Control.Monad (foldM)

data Graph = Graph
    { fwdEdges :: Map String (Set String)
    , revEdges :: Map String (Set String)
    , allowSelfLoops :: Bool
    } deriving (Show, Eq)

new :: Graph
new = Graph Map.empty Map.empty False

newAllowSelfLoops :: Graph
newAllowSelfLoops = Graph Map.empty Map.empty True

addNode :: String -> Graph -> Graph
addNode n g =
    if Map.member n (fwdEdges g)
        then g
        else g { fwdEdges = Map.insert n Set.empty (fwdEdges g)
               , revEdges = Map.insert n Set.empty (revEdges g)
               }

addEdge :: String -> String -> Graph -> Graph
addEdge f t g =
    if f == t && not (allowSelfLoops g)
        then error $ "self-loop not allowed: " ++ show f
        else 
            let g1 = addNode f (addNode t g)
                fwd = Map.adjust (Set.insert t) f (fwdEdges g1)
                rev = Map.adjust (Set.insert f) t (revEdges g1)
            in g1 { fwdEdges = fwd, revEdges = rev }

removeNode :: String -> Graph -> Either String Graph
removeNode n g =
    if not (Map.member n (fwdEdges g))
        then Left $ "Node not found: " ++ n
        else Right $ g 
            { fwdEdges = Map.delete n (Map.map (Set.delete n) (fwdEdges g))
            , revEdges = Map.delete n (Map.map (Set.delete n) (revEdges g))
            }
            
removeEdge :: String -> String -> Graph -> Either String Graph
removeEdge f t g =
    case Map.lookup f (fwdEdges g) of
        Nothing -> Left $ "Edge not found: " ++ f ++ " -> " ++ t
        Just succs -> 
            if not (Set.member t succs)
                then Left $ "Edge not found: " ++ f ++ " -> " ++ t
                else Right $ g 
                    { fwdEdges = Map.adjust (Set.delete t) f (fwdEdges g)
                    , revEdges = Map.adjust (Set.delete f) t (revEdges g)
                    }

hasNode :: String -> Graph -> Bool
hasNode n g = Map.member n (fwdEdges g)

hasEdge :: String -> String -> Graph -> Bool
hasEdge f t g = case Map.lookup f (fwdEdges g) of
    Nothing -> False
    Just succs -> Set.member t succs

nodes :: Graph -> [String]
nodes g = sort $ Map.keys (fwdEdges g)

edges :: Graph -> [(String, String)]
edges g = 
    let es = [ (f, t) | (f, succs) <- Map.toList (fwdEdges g), t <- Set.toList succs ]
    in sort es

predecessors :: String -> Graph -> Either String [String]
predecessors n g =
    case Map.lookup n (revEdges g) of
        Nothing -> Left $ "Node not found: " ++ n
        Just preds -> Right $ sort $ Set.toList preds

successors :: String -> Graph -> Either String [String]
successors n g =
    case Map.lookup n (fwdEdges g) of
        Nothing -> Left $ "Node not found: " ++ n
        Just succs -> Right $ sort $ Set.toList succs

size :: Graph -> Int
size g = Map.size (fwdEdges g)

topologicalSort :: Graph -> Either String [String]
topologicalSort g = 
    let inDegree = Map.map Set.size (revEdges g)
        queue = sort [ n | (n, v) <- Map.toList inDegree, v == 0 ]
        loop [] m res
            | length res == Map.size (fwdEdges g) = Right (reverse res)
            | otherwise = Left "Cycle detected"
        loop (n:ns) m res = 
            let succs = sort $ Set.toList $ fromMaybe Set.empty (Map.lookup n (fwdEdges g))
                (m', newReady) = foldl (\(accM, rdy) s -> 
                    let d = fromMaybe 0 (Map.lookup s accM) - 1
                        accM' = Map.insert s d accM
                    in if d == 0 then (accM', s : rdy) else (accM', rdy)
                    ) (m, []) succs
                q' = sort $ ns ++ newReady
            in loop q' m' (n:res)
    in loop queue inDegree []

hasCycle :: Graph -> Bool
hasCycle g =
    case topologicalSort g of
        Left _ -> True
        Right _ -> False

transitiveClosure :: String -> Graph -> Either String (Set String)
transitiveClosure n g =
    if not (hasNode n g)
        then Left $ "Node not found: " ++ n
        else Right $ dfs (Set.singleton n) [n]
  where
    dfs visited [] = visited
    dfs visited (curr:rest) =
        let succs = Set.toList $ fromMaybe Set.empty (Map.lookup curr (fwdEdges g))
            unvisited = filter (`Set.notMember` visited) succs
            visited' = foldl (flip Set.insert) visited unvisited
        in dfs visited' (rest ++ unvisited)

transitiveDependents :: String -> Graph -> Either String (Set String)
transitiveDependents = transitiveClosure

independentGroups :: Graph -> Either String [[String]]
independentGroups g = 
    let inDegree = Map.map Set.size (revEdges g)
        queue = sort [ n | (n, v) <- Map.toList inDegree, v == 0 ]
        loop [] _ _ acc
            | sum (map length acc) == Map.size (fwdEdges g) = Right (reverse acc)
            | otherwise = Left "Cycle detected"
        loop q m processed acc = 
            let succsList = concatMap (\n -> Set.toList $ fromMaybe Set.empty (Map.lookup n (fwdEdges g))) q
                (m', newReadySet) = foldl (\(accM, rdy) s -> 
                    let d = fromMaybe 0 (Map.lookup s accM) - 1
                        accM' = Map.insert s d accM
                    in if d == 0 then (accM', Set.insert s rdy) else (accM', rdy)
                    ) (m, Set.empty) succsList
                q' = sort $ Set.toList newReadySet
            in loop q' m' (processed + length q) (sort q : acc)
    in if Map.null (fwdEdges g) then Right [] else loop queue inDegree 0 []

affectedNodes :: Set String -> Graph -> Set String
affectedNodes changed g = 
    Set.unions [
        case transitiveDependents n g of
            Right deps -> Set.insert n deps
            Left _ -> Set.empty
    | n <- Set.toList changed, hasNode n g ]
