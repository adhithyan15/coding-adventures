// The CLI-facing glue between a CliBuilder ParseResult's flags/arguments
// dictionaries and Renderer's typed inputs. Kept separate from Program.fs
// so it's directly unit-testable without spawning a process or driving a
// real Parser.
module CodingAdventures.Cowsay.Cli

open System
open System.Collections.Generic
open System.IO
open CodingAdventures.Cowsay.Renderer

let private modeFlagIds =
    [ "borg"; "dead"; "greedy"; "paranoid"; "stoned"; "tired"; "wired"; "youthful" ]

let private getBool (flags: IReadOnlyDictionary<string, obj>) (key: string) : bool =
    match flags.TryGetValue(key) with
    | true, (:? bool as value) -> value
    | _ -> false

let isListRequested (flags: IReadOnlyDictionary<string, obj>) : bool = getBool flags "list"

/// Cow file basenames under `cowsDir`, sorted ordinally.
let listCowFiles (cowsDir: string) : string list =
    Directory.EnumerateFiles(cowsDir, "*.cow")
    |> Seq.map Path.GetFileNameWithoutExtension
    |> Seq.sortWith (fun a b -> String.CompareOrdinal(a, b))
    |> List.ofSeq

/// Resolves the message from the parsed "message" positional argument.
/// Returns None when no message was given on argv — the caller should fall
/// back to stdin.
let resolveMessageFromArguments (arguments: IReadOnlyDictionary<string, obj>) : string option =
    match arguments.TryGetValue("message") with
    | true, (:? ResizeArray<obj> as items) when items.Count > 0 ->
        items
        |> Seq.map (fun item -> if isNull item then "" else item.ToString())
        |> String.concat " "
        |> Some
    | _ -> None

/// Builds a CowsayInvocation from a resolved message and the parsed flags
/// dictionary, applying cowsay.json's documented defaults for any flag that
/// wasn't explicitly set.
let buildInvocation (message: string) (flags: IReadOnlyDictionary<string, obj>) : CowsayInvocation =
    let getString key defaultValue =
        match flags.TryGetValue(key) with
        | true, (:? string as s) -> s
        | _ -> defaultValue

    let width =
        match flags.TryGetValue("width") with
        | true, (:? int64 as w) -> w |> max 1L |> min (int64 Int32.MaxValue) |> int
        | true, (:? int as w) -> max 1 w
        | _ -> 40

    let activeModes = modeFlagIds |> List.filter (getBool flags)

    { Message = message
      Eyes = getString "eyes" "oo"
      Tongue = getString "tongue" "  "
      ActiveModes = activeModes
      NoWrap = getBool flags "nowrap"
      Width = width
      Think = getBool flags "think"
      CowFile = getString "cowfile" "default" }
