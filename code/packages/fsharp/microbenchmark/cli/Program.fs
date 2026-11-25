namespace CodingAdventures.ArrayMicrobenchmark

open System
open System.Globalization
open System.IO
open CodingAdventures.Microbenchmark

module Program =
    let private usage = "Usage: array-microbenchmark [--pairs N] [--confidence P] [--factor F] [--seed N] [--target-ns N] [--max-batch N] [--revision ID]"

    let private parseInt name (text: string) =
        match Int32.TryParse(text, NumberStyles.None, CultureInfo.InvariantCulture) with
        | true, value -> Ok value
        | false, _ -> Error("invalid " + name)

    let private parseInt64 name (text: string) =
        match Int64.TryParse(text, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture) with
        | true, value -> Ok value
        | false, _ -> Error("invalid " + name)

    let private parseFloat name (text: string) =
        match Double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture) with
        | true, value -> Ok value
        | false, _ -> Error("invalid " + name)

    let private parseArgs (arguments: string array) =
        let mutable config = Microbenchmark.defaultConfig
        let mutable revision = Environment.GetEnvironmentVariable("GIT_REVISION")
        if String.IsNullOrWhiteSpace revision then revision <- Environment.GetEnvironmentVariable("GITHUB_SHA")
        if String.IsNullOrWhiteSpace revision then revision <- "unknown"
        let rec loop index =
            if index >= arguments.Length then Ok(config, revision)
            elif arguments[index] = "--help" || arguments[index] = "-h" then Error "HELP"
            elif index + 1 >= arguments.Length then Error(arguments[index] + " requires a value")
            else
                let flag = arguments[index]
                let value = arguments[index + 1]
                let next =
                    match flag with
                    | "--pairs" -> parseInt flag value |> Result.map (fun parsed -> config <- { config with pairs = parsed })
                    | "--confidence" -> parseInt flag value |> Result.map (fun parsed -> config <- { config with confidencePercent = parsed })
                    | "--factor" -> parseFloat flag value |> Result.map (fun parsed -> config <- { config with minimumFactor = parsed })
                    | "--seed" -> parseInt64 flag value |> Result.map (fun parsed -> config <- { config with seed = parsed })
                    | "--target-ns" -> parseInt64 flag value |> Result.map (fun parsed -> config <- { config with targetNs = parsed })
                    | "--max-batch" -> parseInt64 flag value |> Result.map (fun parsed -> config <- { config with maxBatch = parsed })
                    | "--revision" ->
                        if String.IsNullOrWhiteSpace value then Error "--revision must not be blank"
                        else
                            revision <- value
                            Ok()
                    | _ -> Error("unknown option: " + flag)
                match next with
                | Error message -> Error message
                | Ok() -> loop (index + 2)
        loop 0

    [<EntryPoint>]
    let main arguments =
        try
            match parseArgs arguments with
            | Error "HELP" -> Microbenchmark.writeAndFlush Console.Out (usage + Environment.NewLine); 0
            | Error message -> Console.Error.WriteLine("array-microbenchmark: " + message); 1
            | Ok(config, revision) ->
                let executable = Environment.ProcessPath |> Option.ofObj |> Option.defaultValue "array-microbenchmark"
                let invocation = Array.append [| executable |] arguments
#if DEBUG
                let profile = "debug"
#else
                let profile = "release"
#endif
                match ArrayWorkload.run config invocation revision profile (ArrayWorkload.createClock()) with
                | Error error -> Console.Error.WriteLine(sprintf "array-microbenchmark: %A" error); 1
                | Ok report -> Microbenchmark.writeAndFlush Console.Out (Microbenchmark.toJson report + Environment.NewLine); 0
        with error ->
            Console.Error.WriteLine("array-microbenchmark: " + error.Message)
            1
