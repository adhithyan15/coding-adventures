namespace CodingAdventures.IrcProto.FSharp

open System
open System.Text

type Message =
    { Prefix: string option
      Command: string
      Params: string list }

exception ParseError of string

[<RequireQualifiedAccess>]
module IrcProto =
    [<Literal>]
    let Version = "0.1.0"

    let private maxParams = 15

    let parse (line: string) =
        if isNull line then
            nullArg "line"

        if line.Length = 0 || String.IsNullOrWhiteSpace line then
            raise (ParseError $"empty or whitespace-only line: {line}")

        let mutable rest = line

        let prefix =
            if rest.StartsWith ":" then
                let spacePosition = rest.IndexOf ' '

                if spacePosition = -1 then
                    raise (ParseError $"line has prefix but no command: {line}")

                let value = rest.Substring(1, spacePosition - 1)
                rest <- rest.Substring(spacePosition + 1)
                Some value
            else
                None

        let commandRaw =
            let commandEnd = rest.IndexOf ' '

            if commandEnd = -1 then
                let value = rest
                rest <- ""
                value
            else
                let value = rest.Substring(0, commandEnd)
                rest <- rest.Substring(commandEnd + 1)
                value

        let command = commandRaw.ToUpperInvariant()

        if command.Length = 0 then
            raise (ParseError $"could not extract command from line: {line}")

        let parameters = ResizeArray<string>()

        while rest.Length > 0 do
            if rest.StartsWith ":" then
                parameters.Add(rest.Substring 1)
                rest <- ""
            else
                let spacePosition = rest.IndexOf ' '

                if spacePosition = -1 then
                    parameters.Add rest
                    rest <- ""
                else
                    parameters.Add(rest.Substring(0, spacePosition))
                    rest <- rest.Substring(spacePosition + 1)

                    if parameters.Count = maxParams then
                        rest <- ""

        { Prefix = prefix
          Command = command
          Params = parameters |> Seq.toList }

    let serialize (message: Message) =
        let parts = ResizeArray<string>()

        match message.Prefix with
        | Some prefix -> parts.Add($":{prefix}")
        | None -> ()

        parts.Add message.Command

        message.Params
        |> List.iteri (fun index parameter ->
            let isLast = index = message.Params.Length - 1
            parts.Add(if isLast && parameter.Contains " " then $":{parameter}" else parameter))

        String.concat " " parts + "\r\n"
        |> Encoding.UTF8.GetBytes
