namespace CodingAdventures.NeuralGraphVM

open System
open CodingAdventures.NeuralNetwork

type NeuralBytecodeInstruction =
    { Op: string
      Dst: string option
      InputName: string option
      OutputName: string option
      EdgeId: string option
      Value: float option
      Left: string option
      Right: string option
      Inputs: string list
      Input: string option
      Activation: string option
      SourceNode: string option
      SourceEdge: string option }

type NeuralBytecodeFunction = { Id: string; Kind: string; Instructions: NeuralBytecodeInstruction list }
type NeuralBytecodeGraphEdge = { Id: string; From: string; To: string; Weight: float }
type NeuralBytecodeModule = { Magic: string; Version: int; Nodes: string list; Edges: NeuralBytecodeGraphEdge list; Functions: NeuralBytecodeFunction list }

[<RequireQualifiedAccess>]
module NeuralGraphVM =
    let private inst op = { Op = op; Dst = None; InputName = None; OutputName = None; EdgeId = None; Value = None; Left = None; Right = None; Inputs = []; Input = None; Activation = None; SourceNode = None; SourceEdge = None }
    let private stringProp (key: string) (props: PropertyBag) =
        Map.tryFind key props |> Option.bind (fun (value: obj) -> match value with :? string as text -> Some text | _ -> None)

    let private numberProp (key: string) (props: PropertyBag) =
        Map.tryFind key props |> Option.map (fun value -> Convert.ToDouble(value))

    let compileNeuralGraphToBytecode (graph: NeuralGraph) =
        match NeuralGraph.topologicalSort graph with
        | Error err -> Error err
        | Ok order ->
            let compileNode (instructions, values, nextValueId) node =
                let alloc next = sprintf "v%i" next, next + 1
                let props = NeuralGraph.nodeProperties node graph
                match stringProp "nn.op" props |> Option.defaultValue "weighted_sum" with
                | "input" ->
                    let slot, nextValueId = alloc nextValueId
                    instructions @ [ { inst "LOAD_INPUT" with Dst = Some slot; InputName = Some (stringProp "nn.input" props |> Option.defaultValue node); SourceNode = Some node } ], Map.add node slot values, nextValueId
                | "constant" ->
                    let slot, nextValueId = alloc nextValueId
                    instructions @ [ { inst "LOAD_CONST" with Dst = Some slot; Value = numberProp "nn.value" props; SourceNode = Some node } ], Map.add node slot values, nextValueId
                | "weighted_sum" ->
                    let edgeFolder (insts: NeuralBytecodeInstruction list, terms: string list, nextId: int) (edge: NeuralEdge) =
                        let weightValue, nextId = alloc nextId
                        let termValue, nextId = alloc nextId
                        insts @ [ { inst "LOAD_EDGE_WEIGHT" with Dst = Some weightValue; EdgeId = Some edge.Id; SourceEdge = Some edge.Id }; { inst "MUL" with Dst = Some termValue; Left = Map.tryFind edge.From values; Right = Some weightValue; SourceEdge = Some edge.Id } ], terms @ [ termValue ], nextId
                    let termInstructions, terms, nextValueId = NeuralGraph.incomingEdges node graph |> List.sortBy _.Id |> List.fold edgeFolder ([], [], nextValueId)
                    let slot, nextValueId = alloc nextValueId
                    let addInstruction = if List.isEmpty terms then { inst "LOAD_CONST" with Dst = Some slot; Value = Some 0.0; SourceNode = Some node } else { inst "ADD" with Dst = Some slot; Inputs = terms; SourceNode = Some node }
                    instructions @ termInstructions @ [ addInstruction ], Map.add node slot values, nextValueId
                | "activation" ->
                    let slot, nextValueId = alloc nextValueId
                    let edge = NeuralGraph.incomingEdges node graph |> List.exactlyOne
                    instructions @ [ { inst "ACTIVATE" with Dst = Some slot; Input = Map.tryFind edge.From values; Activation = stringProp "nn.activation" props |> Option.orElse (Some "relu"); SourceNode = Some node } ], Map.add node slot values, nextValueId
                | "output" ->
                    let edge = NeuralGraph.incomingEdges node graph |> List.exactlyOne
                    let input = Map.find edge.From values
                    instructions @ [ { inst "STORE_OUTPUT" with OutputName = Some (stringProp "nn.output" props |> Option.defaultValue node); Input = Some input; SourceNode = Some node } ], Map.add node input values, nextValueId
                | other -> failwithf "unsupported neural graph op: %s" other
            let instructions, _, _ = order |> List.fold compileNode ([], Map.empty, 0)
            Ok { Magic = "CANN"; Version = 0; Nodes = graph.Nodes; Edges = graph.Edges |> List.map (fun (edge: NeuralEdge) -> ({ Id = edge.Id; From = edge.From; To = edge.To; Weight = edge.Weight } : NeuralBytecodeGraphEdge)); Functions = [ { Id = "forward"; Kind = "forward"; Instructions = instructions } ] }

    let compileNeuralNetworkToBytecode model = compileNeuralGraphToBytecode model.Graph

    let applyNeuralActivation value activation =
        match activation with
        | "relu" -> if value > 0.0 then value else 0.0
        | "sigmoid" -> 1.0 / (1.0 + Math.Exp(-value))
        | "tanh" -> Math.Tanh(value)
        | _ -> value

    let runNeuralBytecodeForward bytecode inputs =
        let edgeWeights = bytecode.Edges |> List.map (fun edge -> edge.Id, edge.Weight) |> Map.ofList
        let forward = bytecode.Functions |> List.find (fun fn -> fn.Kind = "forward")
        let read values maybeId = maybeId |> Option.bind (fun id -> Map.tryFind id values) |> Option.defaultValue 0.0
        let folder (outputs, values) instruction =
            match instruction.Op with
            | "LOAD_INPUT" -> outputs, Map.add instruction.Dst.Value (Map.find instruction.InputName.Value inputs) values
            | "LOAD_CONST" -> outputs, Map.add instruction.Dst.Value (defaultArg instruction.Value 0.0) values
            | "LOAD_EDGE_WEIGHT" -> outputs, Map.add instruction.Dst.Value (Map.tryFind instruction.EdgeId.Value edgeWeights |> Option.defaultValue 1.0) values
            | "MUL" -> outputs, Map.add instruction.Dst.Value ((read values instruction.Left) * (read values instruction.Right)) values
            | "ADD" -> outputs, Map.add instruction.Dst.Value (instruction.Inputs |> List.sumBy (fun id -> Map.find id values)) values
            | "ACTIVATE" -> outputs, Map.add instruction.Dst.Value (applyNeuralActivation (read values instruction.Input) (defaultArg instruction.Activation "relu")) values
            | "STORE_OUTPUT" -> Map.add (defaultArg instruction.OutputName "output") (read values instruction.Input) outputs, values
            | other -> failwithf "unsupported opcode: %s" other
        forward.Instructions |> List.fold folder (Map.empty, Map.empty) |> fst
