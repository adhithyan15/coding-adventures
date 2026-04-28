namespace CodingAdventures.JsonRpc.FSharp

open System
open System.Collections.Generic
open System.IO
open System.Text
open System.Text.Encodings.Web
open System.Text.Json
open System.Text.Json.Nodes

[<RequireQualifiedAccess>]
module ErrorCodes =
    [<Literal>]
    let PARSE_ERROR = -32700

    [<Literal>]
    let INVALID_REQUEST = -32600

    [<Literal>]
    let METHOD_NOT_FOUND = -32601

    [<Literal>]
    let INVALID_PARAMS = -32602

    [<Literal>]
    let INTERNAL_ERROR = -32603

type JsonRpcException(code: int, message: string) =
    inherit Exception(message)
    member _.Code = code

type ResponseError =
    { Code: int
      Message: string
      Data: JsonNode option }

type Request =
    { Id: JsonNode
      Method: string
      Params: JsonNode option }

type Response =
    { Id: JsonNode option
      Result: JsonNode option
      Error: ResponseError option }

type Notification =
    { Method: string
      Params: JsonNode option }

type Message =
    | RequestMessage of Request
    | ResponseMessage of Response
    | NotificationMessage of Notification

module private Helpers =
    let jsonOptions = JsonSerializerOptions()
    do
        jsonOptions.WriteIndented <- false
        jsonOptions.Encoder <- JavaScriptEncoder.UnsafeRelaxedJsonEscaping

    let strictUtf8 = UTF8Encoding(false, true)

    let fail code message = raise (JsonRpcException(code, message))

    let clone (node: JsonNode) = node.DeepClone()

    let cloneOpt (node: JsonNode option) = node |> Option.map clone

    let item (object: JsonObject) (name: string) : JsonNode = object.Item(name)

    let tryGetString (node: JsonNode) =
        if isNull node then
            None
        else
            match node with
            | :? JsonValue as value ->
                let mutable text = Unchecked.defaultof<string>
                if value.TryGetValue<string>(&text) then Some text else None
            | _ -> None

    let tryGetInt (node: JsonNode) =
        if isNull node then
            None
        else
            match node with
            | :? JsonValue as value ->
                let mutable number = 0
                if value.TryGetValue<int>(&number) then Some number else None
            | _ -> None

    let isValidId (node: JsonNode) =
        if isNull node then
            false
        else
            match node with
            | :? JsonValue as value ->
                let mutable text = Unchecked.defaultof<string>
                let mutable number = 0
                let mutable longNumber = 0L
                value.TryGetValue<string>(&text)
                || value.TryGetValue<int>(&number)
                || value.TryGetValue<int64>(&longNumber)
            | _ -> false

    let responseErrorToNode error =
        let object = JsonObject()
        object.Item("code") <- JsonValue.Create(error.Code)
        object.Item("message") <- JsonValue.Create(error.Message)
        match error.Data with
        | Some data -> object.Item("data") <- clone data
        | None -> ()
        object

    let toJsonNode (value: obj) =
        if isNull value then
            None
        else
            match value with
            | :? JsonNode as node -> Some(clone node)
            | :? string as text -> Some(JsonValue.Create(text) :> JsonNode)
            | :? int as number -> Some(JsonValue.Create(number) :> JsonNode)
            | :? int64 as number -> Some(JsonValue.Create(number) :> JsonNode)
            | :? bool as boolean -> Some(JsonValue.Create(boolean) :> JsonNode)
            | :? double as number -> Some(JsonValue.Create(number) :> JsonNode)
            | :? ResponseError as error -> Some(responseErrorToNode error :> JsonNode)
            | _ -> JsonSerializer.SerializeToNode(value, value.GetType(), jsonOptions) |> Option.ofObj

    let baseObject () =
        let object = JsonObject()
        object.Item("jsonrpc") <- JsonValue.Create("2.0")
        object

    let messageToNode message =
        match message with
        | RequestMessage request ->
            let object = baseObject()
            object.Item("id") <- clone request.Id
            object.Item("method") <- JsonValue.Create(request.Method)
            match request.Params with
            | Some parameters -> object.Item("params") <- clone parameters
            | None -> ()
            object
        | ResponseMessage response ->
            let object = baseObject()
            object.Item("id") <-
                match response.Id with
                | Some id -> clone id
                | None -> null
            match response.Error with
            | Some error -> object.Item("error") <- responseErrorToNode error
            | None ->
                object.Item("result") <-
                    match response.Result with
                    | Some result -> clone result
                    | None -> null
            object
        | NotificationMessage notification ->
            let object = baseObject()
            object.Item("method") <- JsonValue.Create(notification.Method)
            match notification.Params with
            | Some parameters -> object.Item("params") <- clone parameters
            | None -> ()
            object

    let messageToJson message = (messageToNode message).ToJsonString(jsonOptions)

    let parseMessage (raw: string) =
        let node =
            try
                JsonNode.Parse(raw)
            with :? JsonException as ex ->
                fail ErrorCodes.PARSE_ERROR $"Parse error: {ex.Message}"

        match node with
        | :? JsonObject as object ->
            match tryGetString (item object "jsonrpc") with
            | Some "2.0" -> ()
            | _ -> fail ErrorCodes.INVALID_REQUEST "Invalid Request: missing or wrong jsonrpc field"

            if object.ContainsKey("result") || object.ContainsKey("error") then
                let errorNode = item object "error"
                let error =
                    if isNull errorNode then
                        None
                    else
                        match errorNode with
                        | :? JsonObject as errorObject ->
                            let code =
                                match tryGetInt (item errorObject "code") with
                                | Some code -> code
                                | None -> fail ErrorCodes.INVALID_REQUEST "Invalid Request: error code must be an integer"

                            let message = defaultArg (tryGetString (item errorObject "message")) ""
                            Some
                                { Code = code
                                  Message = message
                                  Data = if isNull (item errorObject "data") then None else Some(clone (item errorObject "data")) }
                        | _ -> fail ErrorCodes.INVALID_REQUEST "Invalid Request: error must be a JSON object"

                ResponseMessage
                    { Id = if isNull (item object "id") then None else Some(clone (item object "id"))
                      Result = if isNull (item object "result") then None else Some(clone (item object "result"))
                      Error = error }
            else
                let method =
                    match tryGetString (item object "method") with
                    | Some method -> method
                    | None -> fail ErrorCodes.INVALID_REQUEST "Invalid Request: method must be a string"

                let parameters = if isNull (item object "params") then None else Some(clone (item object "params"))

                if object.ContainsKey("id") then
                    let id = item object "id"
                    if not (isValidId id) then
                        fail ErrorCodes.INVALID_REQUEST "Invalid Request: id must be a string or integer"

                    RequestMessage
                        { Id = clone id
                          Method = method
                          Params = parameters }
                else
                    NotificationMessage { Method = method; Params = parameters }
        | _ -> fail ErrorCodes.INVALID_REQUEST "Invalid Request: top-level value must be a JSON object"

    let readLineBytes (stream: Stream) =
        let bytes = ResizeArray<byte>()
        let mutable keepReading = true

        while keepReading do
            let value = stream.ReadByte()
            if value < 0 then
                keepReading <- false
            else
                bytes.Add(byte value)
                if value = int '\n' then
                    keepReading <- false

        if bytes.Count = 0 then None else Some(bytes.ToArray())

    let readExactly (stream: Stream) count =
        let buffer = Array.zeroCreate<byte> count
        let mutable offset = 0
        while offset < count do
            let read = stream.Read(buffer, offset, count - offset)
            if read = 0 then
                fail ErrorCodes.PARSE_ERROR $"Parse error: expected {count} bytes but stream ended after {offset}"
            offset <- offset + read
        buffer

type MessageReader(stream: Stream) =
    do
        if isNull stream then nullArg "stream"

    member _.ReadRaw() =
        let mutable contentLength: int option = None
        let mutable readingHeaders = true
        let mutable eofBetweenMessages = false

        while readingHeaders do
            match Helpers.readLineBytes stream with
            | None ->
                if Option.isNone contentLength then
                    eofBetweenMessages <- true
                    readingHeaders <- false
                else
                    Helpers.fail ErrorCodes.PARSE_ERROR "Parse error: unexpected EOF in header block"
            | Some lineBytes ->
                let line = Encoding.ASCII.GetString(lineBytes).TrimEnd([| '\r'; '\n' |])
                if line.Length = 0 then
                    readingHeaders <- false
                else
                    let separator = line.IndexOf(':')
                    if separator >= 0 then
                        let name = line.Substring(0, separator).Trim()
                        let value = line.Substring(separator + 1).Trim()
                        if name.Equals("Content-Length", StringComparison.OrdinalIgnoreCase) then
                            let mutable parsed = 0
                            if Int32.TryParse(value, &parsed) then
                                contentLength <- Some parsed
                            else
                                Helpers.fail ErrorCodes.PARSE_ERROR $"Parse error: invalid Content-Length value: {value}"

        if eofBetweenMessages then
            None
        else
            match contentLength with
            | None -> Helpers.fail ErrorCodes.PARSE_ERROR "Parse error: no Content-Length header found"
            | Some length when length < 0 ->
                Helpers.fail ErrorCodes.PARSE_ERROR $"Parse error: Content-Length must be non-negative, got {length}"
            | Some length ->
                let payload = Helpers.readExactly stream length
                try
                    Some(Helpers.strictUtf8.GetString(payload))
                with :? DecoderFallbackException as ex ->
                    Helpers.fail ErrorCodes.PARSE_ERROR $"Parse error: payload is not valid UTF-8: {ex.Message}"

    member this.ReadMessage() =
        match this.ReadRaw() with
        | None -> None
        | Some raw -> Some(Helpers.parseMessage raw)

type MessageWriter(stream: Stream) =
    do
        if isNull stream then nullArg "stream"

    member _.WriteRaw(json: string) =
        if isNull json then nullArg "json"
        let payload = Encoding.UTF8.GetBytes(json)
        let header = Encoding.ASCII.GetBytes($"Content-Length: {payload.Length}\r\n\r\n")
        stream.Write(header, 0, header.Length)
        stream.Write(payload, 0, payload.Length)
        stream.Flush()

    member this.WriteMessage(message: Message) =
        this.WriteRaw(Helpers.messageToJson message)

type RequestHandler = JsonNode -> JsonNode option -> obj

type NotificationHandler = JsonNode option -> unit

type Server(input: Stream, output: Stream) =
    let reader = MessageReader(input)
    let writer = MessageWriter(output)
    let requestHandlers = Dictionary<string, RequestHandler>()
    let notificationHandlers = Dictionary<string, NotificationHandler>()

    member this.OnRequest(methodName: string, handler: RequestHandler) =
        if isNull methodName then nullArg "methodName"
        if isNull (box handler) then nullArg "handler"
        requestHandlers[methodName] <- handler
        this

    member this.OnNotification(methodName: string, handler: NotificationHandler) =
        if isNull methodName then nullArg "methodName"
        if isNull (box handler) then nullArg "handler"
        notificationHandlers[methodName] <- handler
        this

    member this.Serve() =
        let mutable running = true
        while running do
            try
                match reader.ReadMessage() with
                | None -> running <- false
                | Some message -> this.Dispatch message
            with :? JsonRpcException as ex ->
                this.SendError(None, ex.Code, ex.Message)

    member private this.Dispatch(message: Message) =
        match message with
        | RequestMessage request -> this.HandleRequest request
        | NotificationMessage notification -> this.HandleNotification notification
        | ResponseMessage _ -> ()

    member private this.HandleRequest(request: Request) =
        let mutable handler = Unchecked.defaultof<RequestHandler>
        if not (requestHandlers.TryGetValue(request.Method, &handler)) then
            this.SendError(Some request.Id, ErrorCodes.METHOD_NOT_FOUND, "Method not found")
        else
            try
                let result = handler request.Id request.Params
                let response =
                    match result with
                    | :? ResponseError as error ->
                        ResponseMessage
                            { Id = Some(Helpers.clone request.Id)
                              Result = None
                              Error = Some error }
                    | _ ->
                        ResponseMessage
                            { Id = Some(Helpers.clone request.Id)
                              Result = Helpers.toJsonNode result
                              Error = None }

                writer.WriteMessage response
            with ex ->
                this.SendError(Some request.Id, ErrorCodes.INTERNAL_ERROR, $"Internal error: {ex.Message}")

    member private _.HandleNotification(notification: Notification) =
        let mutable handler = Unchecked.defaultof<NotificationHandler>
        if notificationHandlers.TryGetValue(notification.Method, &handler) then
            try
                handler notification.Params
            with _ ->
                ()

    member private _.SendError(id: JsonNode option, code: int, message: string) =
        writer.WriteMessage(
            ResponseMessage
                { Id = Helpers.cloneOpt id
                  Result = None
                  Error = Some { Code = code; Message = message; Data = None } }
        )

[<RequireQualifiedAccess>]
module JsonRpc =
    [<Literal>]
    let VERSION = "0.1.0"

    [<Literal>]
    let PARSE_ERROR = ErrorCodes.PARSE_ERROR

    [<Literal>]
    let INVALID_REQUEST = ErrorCodes.INVALID_REQUEST

    [<Literal>]
    let METHOD_NOT_FOUND = ErrorCodes.METHOD_NOT_FOUND

    [<Literal>]
    let INVALID_PARAMS = ErrorCodes.INVALID_PARAMS

    [<Literal>]
    let INTERNAL_ERROR = ErrorCodes.INTERNAL_ERROR

    let parseMessage raw = Helpers.parseMessage raw

    let messageToNode message = Helpers.messageToNode message
