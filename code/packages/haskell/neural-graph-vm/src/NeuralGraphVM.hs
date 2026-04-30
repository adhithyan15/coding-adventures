module NeuralGraphVM
    ( Instruction(..)
    , BytecodeFunction(..)
    , BytecodeGraphEdge(..)
    , BytecodeModule(..)
    , compileNeuralGraphToBytecode
    , compileNeuralNetworkToBytecode
    , runNeuralBytecodeForward
    , applyNeuralActivation
    ) where

import qualified Data.Map.Strict as Map
import Data.List (sortOn)

import NeuralNetwork

data Instruction = Instruction
    { op :: String
    , dst :: Maybe String
    , inputName :: Maybe String
    , outputName :: Maybe String
    , instructionEdgeId :: Maybe String
    , value :: Maybe Double
    , left :: Maybe String
    , right :: Maybe String
    , instructionInputs :: [String]
    , input :: Maybe String
    , activation :: Maybe String
    , sourceNode :: Maybe String
    , sourceEdge :: Maybe String
    } deriving (Eq, Show)

data BytecodeFunction = BytecodeFunction { functionId :: String, functionKind :: String, instructions :: [Instruction] }
    deriving (Eq, Show)

data BytecodeGraphEdge = BytecodeGraphEdge { bytecodeEdgeId :: String, bytecodeEdgeFrom :: String, bytecodeEdgeTo :: String, bytecodeEdgeWeight :: Double }
    deriving (Eq, Show)

data BytecodeModule = BytecodeModule { magic :: String, version :: Int, moduleNodes :: [String], moduleEdges :: [BytecodeGraphEdge], functions :: [BytecodeFunction] }
    deriving (Eq, Show)

compileNeuralNetworkToBytecode :: NeuralNetwork -> Either String BytecodeModule
compileNeuralNetworkToBytecode = compileNeuralGraphToBytecode . networkGraph

compileNeuralGraphToBytecode :: NeuralGraph -> Either String BytecodeModule
compileNeuralGraphToBytecode graph = do
    order <- topologicalSort graph
    let (instructionsOut, _, _) = foldl compileNode ([], Map.empty, 0 :: Int) order
    Right BytecodeModule
        { magic = "CANN"
        , version = 0
        , moduleNodes = graphNodes graph
        , moduleEdges = [BytecodeGraphEdge (edgeId edge) (edgeFrom edge) (edgeTo edge) (edgeWeight edge) | edge <- graphEdges graph]
        , functions = [BytecodeFunction "forward" "forward" instructionsOut]
        }
  where
    alloc next = ("v" ++ show next, next + 1)
    compileNode (insts, values, nextId) node =
        case Map.findWithDefault (PString "weighted_sum") "nn.op" props of
            PString "input" ->
                let (slot, nextId') = alloc nextId
                    inputSlot = case Map.lookup "nn.input" props of Just (PString name) -> name; _ -> node
                in (insts ++ [(baseInst "LOAD_INPUT") { dst = Just slot, inputName = Just inputSlot, sourceNode = Just node }], Map.insert node slot values, nextId')
            PString "constant" ->
                let (slot, nextId') = alloc nextId
                    constantValue = case Map.lookup "nn.value" props of Just (PNumber number) -> number; _ -> 0.0
                in (insts ++ [(baseInst "LOAD_CONST") { dst = Just slot, value = Just constantValue, sourceNode = Just node }], Map.insert node slot values, nextId')
            PString "weighted_sum" ->
                let (termInsts, terms, nextAfterTerms) = foldl (compileEdge values) ([], [], nextId) (sortOn edgeId (incomingEdges node graph))
                    (slot, nextId') = alloc nextAfterTerms
                    addInst = if null terms then (baseInst "LOAD_CONST") { dst = Just slot, value = Just 0.0, sourceNode = Just node } else (baseInst "ADD") { dst = Just slot, instructionInputs = terms, sourceNode = Just node }
                in (insts ++ termInsts ++ [addInst], Map.insert node slot values, nextId')
            PString "activation" ->
                let (slot, nextId') = alloc nextId
                    activationName = case Map.lookup "nn.activation" props of Just (PString name) -> name; _ -> "relu"
                    inputSlot = singleInputValue graph values node
                in (insts ++ [(baseInst "ACTIVATE") { dst = Just slot, input = inputSlot, activation = Just activationName, sourceNode = Just node }], Map.insert node slot values, nextId')
            PString "output" ->
                let outputSlot = case Map.lookup "nn.output" props of Just (PString name) -> name; _ -> node
                    inputSlot = singleInputValue graph values node
                in (insts ++ [(baseInst "STORE_OUTPUT") { outputName = Just outputSlot, input = inputSlot, sourceNode = Just node }], Map.insert node (maybe "" id inputSlot) values, nextId)
            _ -> (insts, values, nextId)
      where
        props = Map.findWithDefault emptyBag node (graphNodeProperties graph)
    compileEdge values (insts, terms, nextId) edge =
        let (weightSlot, nextId1) = alloc nextId
            (termSlot, nextId2) = alloc nextId1
            sourceSlot = Map.lookup (edgeFrom edge) values
        in ( insts ++ [ (baseInst "LOAD_EDGE_WEIGHT") { dst = Just weightSlot, instructionEdgeId = Just (edgeId edge), sourceEdge = Just (edgeId edge) }
                      , (baseInst "MUL") { dst = Just termSlot, left = sourceSlot, right = Just weightSlot, sourceEdge = Just (edgeId edge) }
                      ]
           , terms ++ [termSlot]
           , nextId2
           )

baseInst :: String -> Instruction
baseInst opcode = Instruction opcode Nothing Nothing Nothing Nothing Nothing Nothing Nothing [] Nothing Nothing Nothing Nothing

singleInputValue :: NeuralGraph -> Map.Map String String -> String -> Maybe String
singleInputValue graph values node = case incomingEdges node graph of
    [edge] -> Map.lookup (edgeFrom edge) values
    _ -> Nothing

runNeuralBytecodeForward :: BytecodeModule -> Map.Map String Double -> Either String (Map.Map String Double)
runNeuralBytecodeForward bytecode runtimeInputs = do
    forward <- case filter ((== "forward") . functionKind) (functions bytecode) of
        fn:_ -> Right fn
        [] -> Left "neural bytecode module has no forward function"
    let edgeWeights = Map.fromList [(bytecodeEdgeId edge, bytecodeEdgeWeight edge) | edge <- moduleEdges bytecode]
    fmap fst (foldl (execute edgeWeights) (Right (Map.empty, Map.empty)) (instructions forward))
  where
    execute _ (Left err) _ = Left err
    execute edgeWeights (Right (outputs, values)) inst = case op inst of
        "LOAD_INPUT" -> write values outputs (required (dst inst)) (Map.findWithDefault 0.0 (required (inputName inst)) runtimeInputs)
        "LOAD_CONST" -> write values outputs (required (dst inst)) (maybe 0.0 id (value inst))
        "LOAD_EDGE_WEIGHT" -> write values outputs (required (dst inst)) (Map.findWithDefault 1.0 (required (instructionEdgeId inst)) edgeWeights)
        "MUL" -> write values outputs (required (dst inst)) (readValue values (left inst) * readValue values (right inst))
        "ADD" -> write values outputs (required (dst inst)) (sum [Map.findWithDefault 0.0 slot values | slot <- instructionInputs inst])
        "ACTIVATE" -> write values outputs (required (dst inst)) (applyNeuralActivation (readValue values (input inst)) (maybe "relu" id (activation inst)))
        "STORE_OUTPUT" -> Right (Map.insert (maybe "output" id (outputName inst)) (readValue values (input inst)) outputs, values)
        other -> Left ("unsupported opcode: " ++ other)
    write values outputs slot number = Right (outputs, Map.insert slot number values)
    required = maybe "" id
    readValue values maybeSlot = Map.findWithDefault 0.0 (required maybeSlot) values

applyNeuralActivation :: Double -> String -> Double
applyNeuralActivation value activationName = case activationName of
    "relu" -> if value > 0 then value else 0.0
    "sigmoid" -> 1.0 / (1.0 + exp (-value))
    "tanh" -> tanh value
    _ -> value
