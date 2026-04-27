namespace CodingAdventures.TypeCheckerProtocol.FSharp

open System
open System.Collections.Generic
open System.Text.RegularExpressions

type TypeErrorDiagnostic =
    { Message: string
      Line: int
      Column: int }

type TypeCheckResult<'Ast> =
    { TypedAst: 'Ast
      Errors: TypeErrorDiagnostic list }

    member this.Ok = List.isEmpty this.Errors

type ITypeChecker<'AstIn, 'AstOut> =
    abstract Check: 'AstIn -> TypeCheckResult<'AstOut>

[<Sealed>]
type NotHandled private () =
    static member val Value = NotHandled()

[<AbstractClass>]
type GenericTypeChecker<'Ast>() =
    let errors = ResizeArray<TypeErrorDiagnostic>()
    let hooks = Dictionary<string, ResizeArray<'Ast -> obj array -> obj>>()

    static member NormalizeKind(kind: string option) =
        match kind with
        | None
        | Some "" -> ""
        | Some value -> Regex.Replace(value, @"\W+", "_").Trim('_')

    member _.RegisterHook(phase: string, kind: string, hook: 'Ast -> obj array -> obj) =
        if isNull phase then nullArg "phase"
        if isNull kind then nullArg "kind"
        if isNull (box hook) then nullArg "hook"

        let key = $"{phase}:{GenericTypeChecker<'Ast>.NormalizeKind(Some kind)}"

        let bucket =
            match hooks.TryGetValue key with
            | true, existing -> existing
            | false, _ ->
                let created = ResizeArray()
                hooks[key] <- created
                created

        bucket.Add hook

    member this.Dispatch(phase: string, node: 'Ast, ?args: obj array, ?defaultValue: obj) =
        if isNull phase then nullArg "phase"

        let args = defaultArg args [||]
        let defaultValue = defaultArg defaultValue null
        let normalized = this.NodeKind node |> GenericTypeChecker<'Ast>.NormalizeKind
        let keys = [ $"{phase}:{normalized}"; $"{phase}:*" ]
        let mutable handled = false
        let mutable output = defaultValue

        for key in keys do
            if not handled then
                match hooks.TryGetValue key with
                | true, bucket ->
                    for hook in bucket do
                        if not handled then
                            let result = hook node args
                            if not (Object.ReferenceEquals(result, NotHandled.Value)) then
                                output <- result
                                handled <- true
                | false, _ -> ()

        output

    member this.Check(ast: 'Ast) =
        errors.Clear()
        this.Run ast

        { TypedAst = ast
          Errors = errors |> Seq.toList }

    member this.Error(message: string, subject: obj) =
        let line, column = this.Locate subject
        errors.Add({ Message = message; Line = line; Column = column })

    abstract Run: 'Ast -> unit
    abstract NodeKind: 'Ast -> string option
    abstract Locate: obj -> int * int
    default _.Locate(_subject: obj) = 1, 1

    interface ITypeChecker<'Ast, 'Ast> with
        member this.Check ast = this.Check ast

[<RequireQualifiedAccess>]
module TypeCheckerProtocolPackage =
    [<Literal>]
    let Version = "0.1.0"
