module NeuralNetwork
    ( PropertyValue(..)
    , PropertyBag
    , Edge(..)
    , WeightedInput(..)
    , NeuralGraph(..)
    , NeuralNetwork(..)
    , emptyBag
    , stringProp
    , numberProp
    , wi
    , createNeuralGraph
    , createNeuralNetwork
    , addInput
    , addConstant
    , addWeightedSum
    , addActivation
    , addOutput
    , topologicalSort
    , incomingEdges
    , createXorNetwork
    ) where

import qualified Data.Map.Strict as Map
import Data.List (sort)

import Prelude hiding (lookup)

data PropertyValue = PString String | PNumber Double | PBool Bool | PNull
    deriving (Eq, Show)

type PropertyBag = Map.Map String PropertyValue

data Edge = Edge
    { edgeId :: String
    , edgeFrom :: String
    , edgeTo :: String
    , edgeWeight :: Double
    , edgeProperties :: PropertyBag
    } deriving (Eq, Show)

data WeightedInput = WeightedInput
    { weightedFrom :: String
    , weightedWeight :: Double
    , weightedEdgeId :: Maybe String
    , weightedProperties :: PropertyBag
    } deriving (Eq, Show)

data NeuralGraph = NeuralGraph
    { graphProperties :: PropertyBag
    , graphNodes :: [String]
    , graphNodeProperties :: Map.Map String PropertyBag
    , graphEdges :: [Edge]
    , graphNextEdgeId :: Int
    } deriving (Eq, Show)

newtype NeuralNetwork = NeuralNetwork { networkGraph :: NeuralGraph }
    deriving (Eq, Show)

emptyBag :: PropertyBag
emptyBag = Map.empty

stringProp :: String -> String -> PropertyBag
stringProp key value = Map.singleton key (PString value)

numberProp :: String -> Double -> PropertyBag
numberProp key value = Map.singleton key (PNumber value)

wi :: String -> Double -> String -> WeightedInput
wi from weight edge = WeightedInput from weight (Just edge) emptyBag

createNeuralGraph :: Maybe String -> NeuralGraph
createNeuralGraph name =
    NeuralGraph
        { graphProperties = maybe base (\value -> Map.insert "nn.name" (PString value) base) name
        , graphNodes = []
        , graphNodeProperties = Map.empty
        , graphEdges = []
        , graphNextEdgeId = 0
        }
  where
    base = Map.singleton "nn.version" (PString "0")

createNeuralNetwork :: Maybe String -> NeuralNetwork
createNeuralNetwork name = NeuralNetwork (createNeuralGraph name)

addNode :: String -> PropertyBag -> NeuralGraph -> NeuralGraph
addNode node properties graph =
    graph
        { graphNodes = if Map.member node (graphNodeProperties graph) then graphNodes graph else graphNodes graph ++ [node]
        , graphNodeProperties = Map.insertWith Map.union node properties (graphNodeProperties graph)
        }

addEdge :: String -> String -> Double -> PropertyBag -> Maybe String -> NeuralGraph -> (NeuralGraph, String)
addEdge from to weight properties maybeEdgeId graph =
    ( graph2
        { graphEdges = graphEdges graph2 ++ [Edge edge from to weight (Map.insert "weight" (PNumber weight) properties)]
        , graphNextEdgeId = nextId
        }
    , edge
    )
  where
    graph1 = addNode to emptyBag (addNode from emptyBag graph)
    (edge, nextId) = case maybeEdgeId of
        Just value -> (value, graphNextEdgeId graph1)
        Nothing -> ("e" ++ show (graphNextEdgeId graph1), graphNextEdgeId graph1 + 1)
    graph2 = graph1

addInput :: String -> String -> PropertyBag -> NeuralGraph -> NeuralGraph
addInput node inputName properties =
    addNode node (properties `Map.union` Map.fromList [("nn.op", PString "input"), ("nn.input", PString inputName)])

addConstant :: String -> Double -> PropertyBag -> NeuralGraph -> NeuralGraph
addConstant node value properties =
    addNode node (properties `Map.union` Map.fromList [("nn.op", PString "constant"), ("nn.value", PNumber value)])

addWeightedSum :: String -> [WeightedInput] -> PropertyBag -> NeuralGraph -> NeuralGraph
addWeightedSum node inputs properties graph =
    foldl addOne graph1 inputs
  where
    graph1 = addNode node (properties `Map.union` Map.singleton "nn.op" (PString "weighted_sum")) graph
    addOne acc input = fst (addEdge (weightedFrom input) node (weightedWeight input) (weightedProperties input) (weightedEdgeId input) acc)

addActivation :: String -> String -> String -> PropertyBag -> Maybe String -> NeuralGraph -> NeuralGraph
addActivation node input activation properties edgeId graph =
    fst (addEdge input node 1.0 emptyBag edgeId graph1)
  where
    graph1 = addNode node (properties `Map.union` Map.fromList [("nn.op", PString "activation"), ("nn.activation", PString activation)]) graph

addOutput :: String -> String -> String -> PropertyBag -> Maybe String -> NeuralGraph -> NeuralGraph
addOutput node input outputName properties edgeId graph =
    fst (addEdge input node 1.0 emptyBag edgeId graph1)
  where
    graph1 = addNode node (properties `Map.union` Map.fromList [("nn.op", PString "output"), ("nn.output", PString outputName)]) graph

incomingEdges :: String -> NeuralGraph -> [Edge]
incomingEdges node graph = filter ((== node) . edgeTo) (graphEdges graph)

topologicalSort :: NeuralGraph -> Either String [String]
topologicalSort graph = go indegree0 ready0 []
  where
    indegreeBase = Map.fromList [(node, 0 :: Int) | node <- graphNodes graph]
    indegree0 = foldl inc indegreeBase (graphEdges graph)
    inc acc edge = Map.insertWith (+) (edgeTo edge) 1 (Map.insertWith (+) (edgeFrom edge) 0 acc)
    ready0 = sort [node | (node, degree) <- Map.toList indegree0, degree == 0]
    go indegree [] order
        | length order == Map.size indegree = Right order
        | otherwise = Left "neural graph contains a cycle"
    go indegree (node:ready) order = go indegree' (ready ++ sort released) (order ++ [node])
      where
        outgoing = filter ((== node) . edgeFrom) (graphEdges graph)
        (indegree', released) = foldl release (indegree, []) outgoing
        release (degrees, nodes) edge =
            let next = Map.adjust (subtract 1) (edgeTo edge) degrees
            in if Map.findWithDefault 0 (edgeTo edge) next == 0
               then (next, nodes ++ [edgeTo edge])
               else (next, nodes)

createXorNetwork :: String -> NeuralNetwork
createXorNetwork name = NeuralNetwork graph
  where
    graph =
        addOutput "out" "out_activation" "prediction" (stringProp "nn.layer" "output") (Just "activation_to_out") $
        addActivation "out_activation" "out_sum" "sigmoid" (stringProp "nn.layer" "output") (Just "out_sum_to_activation") $
        addWeightedSum "out_sum" [wi "h_or" 20 "h_or_to_out", wi "h_nand" 20 "h_nand_to_out", wi "bias" (-30) "bias_to_out"] (stringProp "nn.layer" "output") $
        addActivation "h_nand" "h_nand_sum" "sigmoid" (stringProp "nn.layer" "hidden") (Just "h_nand_sum_to_h_nand") $
        addWeightedSum "h_nand_sum" [wi "x0" (-20) "x0_to_h_nand", wi "x1" (-20) "x1_to_h_nand", wi "bias" 30 "bias_to_h_nand"] (stringProp "nn.layer" "hidden") $
        addActivation "h_or" "h_or_sum" "sigmoid" (stringProp "nn.layer" "hidden") (Just "h_or_sum_to_h_or") $
        addWeightedSum "h_or_sum" [wi "x0" 20 "x0_to_h_or", wi "x1" 20 "x1_to_h_or", wi "bias" (-10) "bias_to_h_or"] (stringProp "nn.layer" "hidden") $
        addConstant "bias" 1.0 (stringProp "nn.role" "bias") $
        addInput "x1" "x1" emptyBag $
        addInput "x0" "x0" emptyBag $
        createNeuralGraph (Just name)
