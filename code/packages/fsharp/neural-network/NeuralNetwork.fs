namespace CodingAdventures.NeuralNetwork

open System

type PropertyBag = Map<string, obj>

type NeuralEdge =
    { Id: string
      From: string
      To: string
      Weight: float
      Properties: PropertyBag }

type WeightedInput =
    { From: string
      Weight: float
      EdgeId: string option
      Properties: PropertyBag }

[<RequireQualifiedAccess>]
module WeightedInput =
    let create from weight edgeId =
        { From = from; Weight = weight; EdgeId = Some edgeId; Properties = Map.empty }

type NeuralGraph =
    { GraphProperties: PropertyBag
      Nodes: string list
      NodeProperties: Map<string, PropertyBag>
      Edges: NeuralEdge list
      NextEdgeId: int }

type NeuralNetworkModel = { Graph: NeuralGraph }

[<RequireQualifiedAccess>]
module NeuralGraph =
    let create name =
        let graphProperties =
            match name with
            | Some value -> Map.ofList [ "nn.version", box "0"; "nn.name", box value ]
            | None -> Map.ofList [ "nn.version", box "0" ]
        { GraphProperties = graphProperties; Nodes = []; NodeProperties = Map.empty; Edges = []; NextEdgeId = 0 }

    let addNode node (properties: PropertyBag) graph =
        let exists = Map.containsKey node graph.NodeProperties
        let nodes = if exists then graph.Nodes else graph.Nodes @ [ node ]
        let current = Map.tryFind node graph.NodeProperties |> Option.defaultValue Map.empty
        { graph with Nodes = nodes; NodeProperties = Map.add node (Map.fold (fun acc key value -> Map.add key value acc) current properties) graph.NodeProperties }

    let nodeProperties node graph = Map.tryFind node graph.NodeProperties |> Option.defaultValue Map.empty

    let addEdge from toNode weight (properties: PropertyBag) edgeId graph =
        let graph = graph |> addNode from Map.empty |> addNode toNode Map.empty
        let id, nextEdgeId =
            match edgeId with
            | Some value -> value, graph.NextEdgeId
            | None -> sprintf "e%i" graph.NextEdgeId, graph.NextEdgeId + 1
        let props = Map.add "weight" (box weight) properties
        let edge = { Id = id; From = from; To = toNode; Weight = weight; Properties = props }
        { graph with Edges = graph.Edges @ [ edge ]; NextEdgeId = nextEdgeId }, id

    let incomingEdges node graph = graph.Edges |> List.filter (fun edge -> edge.To = node)

    let topologicalSort graph =
        let indegree0 = graph.Nodes |> List.map (fun node -> node, 0) |> Map.ofList
        let indegree =
            graph.Edges
            |> List.fold (fun acc edge ->
                acc
                |> fun m -> if Map.containsKey edge.From m then m else Map.add edge.From 0 m
                |> fun m -> Map.add edge.To ((Map.tryFind edge.To m |> Option.defaultValue 0) + 1) m) indegree0
        let ready = indegree |> Map.toList |> List.choose (fun (node, degree) -> if degree = 0 then Some node else None) |> List.sort
        let rec loop indegree ready order =
            match ready with
            | [] -> if List.length order = Map.count indegree then Ok order else Error "neural graph contains a cycle"
            | node :: rest ->
                let outgoing = graph.Edges |> List.filter (fun edge -> edge.From = node)
                let indegree, released =
                    outgoing
                    |> List.fold (fun (degrees, released) edge ->
                        let nextDegree = (Map.find edge.To degrees) - 1
                        let degrees = Map.add edge.To nextDegree degrees
                        if nextDegree = 0 then degrees, released @ [ edge.To ] else degrees, released) (indegree, [])
                loop indegree (rest @ List.sort released) (order @ [ node ])
        loop indegree ready []

[<RequireQualifiedAccess>]
module NeuralNetwork =
    let createNeuralGraph name = NeuralGraph.create name
    let createNeuralNetwork name = { Graph = createNeuralGraph name }
    let wi from weight edgeId = WeightedInput.create from weight edgeId

    let merge (properties: PropertyBag) additions =
        additions |> List.fold (fun acc (key, value) -> Map.add key value acc) properties

    let addInput graph node inputName properties =
        graph |> NeuralGraph.addNode node (merge properties [ "nn.op", box "input"; "nn.input", box inputName ])

    let addConstant graph node value properties =
        if Double.IsFinite(value) |> not then invalidArg "value" "constant value must be finite"
        graph |> NeuralGraph.addNode node (merge properties [ "nn.op", box "constant"; "nn.value", box value ])

    let addWeightedSum graph node inputs properties =
        let graph = graph |> NeuralGraph.addNode node (merge properties [ "nn.op", box "weighted_sum" ])
        inputs |> List.fold (fun acc input -> NeuralGraph.addEdge input.From node input.Weight input.Properties input.EdgeId acc |> fst) graph

    let addActivation graph node input activation properties edgeId =
        let graph = graph |> NeuralGraph.addNode node (merge properties [ "nn.op", box "activation"; "nn.activation", box activation ])
        NeuralGraph.addEdge input node 1.0 Map.empty edgeId graph |> fst

    let addOutput graph node input outputName properties edgeId =
        let graph = graph |> NeuralGraph.addNode node (merge properties [ "nn.op", box "output"; "nn.output", box outputName ])
        NeuralGraph.addEdge input node 1.0 Map.empty edgeId graph |> fst

    let private setGraph model graph = { model with Graph = graph }
    let input node model = setGraph model (addInput model.Graph node node Map.empty)
    let constant node value properties model = setGraph model (addConstant model.Graph node value properties)
    let weightedSum node inputs properties model = setGraph model (addWeightedSum model.Graph node inputs properties)
    let activation node inputNode activationName properties edgeId model = setGraph model (addActivation model.Graph node inputNode activationName properties edgeId)
    let output node inputNode outputName properties edgeId model = setGraph model (addOutput model.Graph node inputNode outputName properties edgeId)

    let createXorNetwork name =
        createNeuralNetwork (Some name)
        |> input "x0"
        |> input "x1"
        |> constant "bias" 1.0 (Map.ofList [ "nn.role", box "bias" ])
        |> weightedSum "h_or_sum" [ wi "x0" 20.0 "x0_to_h_or"; wi "x1" 20.0 "x1_to_h_or"; wi "bias" -10.0 "bias_to_h_or" ] (Map.ofList [ "nn.layer", box "hidden" ])
        |> activation "h_or" "h_or_sum" "sigmoid" (Map.ofList [ "nn.layer", box "hidden" ]) (Some "h_or_sum_to_h_or")
        |> weightedSum "h_nand_sum" [ wi "x0" -20.0 "x0_to_h_nand"; wi "x1" -20.0 "x1_to_h_nand"; wi "bias" 30.0 "bias_to_h_nand" ] (Map.ofList [ "nn.layer", box "hidden" ])
        |> activation "h_nand" "h_nand_sum" "sigmoid" (Map.ofList [ "nn.layer", box "hidden" ]) (Some "h_nand_sum_to_h_nand")
        |> weightedSum "out_sum" [ wi "h_or" 20.0 "h_or_to_out"; wi "h_nand" 20.0 "h_nand_to_out"; wi "bias" -30.0 "bias_to_out" ] (Map.ofList [ "nn.layer", box "output" ])
        |> activation "out_activation" "out_sum" "sigmoid" (Map.ofList [ "nn.layer", box "output" ]) (Some "out_sum_to_activation")
        |> output "out" "out_activation" "prediction" (Map.ofList [ "nn.layer", box "output" ]) (Some "activation_to_out")
