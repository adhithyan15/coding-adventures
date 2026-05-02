namespace CodingAdventures.SqlCsvSource.FSharp

open System
open System.Collections.Generic
open System.Globalization
open System.IO
open System.Text
open CodingAdventures.CsvParser
open CodingAdventures.SqlExecutionEngine.FSharp

module private CsvHeader =
    let parse (source: string) =
        let fields = ResizeArray<string>()
        let field = StringBuilder()
        let mutable quoted = false
        let mutable afterQuote = false
        let mutable index = 0
        let mutable doneRecord = false

        while index < source.Length && not doneRecord do
            let ch = source[index]

            if quoted then
                if ch = '"' then
                    if index + 1 < source.Length && source[index + 1] = '"' then
                        field.Append('"') |> ignore
                        index <- index + 1
                    else
                        quoted <- false
                        afterQuote <- true
                else
                    field.Append(ch) |> ignore
            elif ch = ',' then
                fields.Add(field.ToString().Trim())
                field.Clear() |> ignore
                afterQuote <- false
            elif ch = '"' && field.Length = 0 && not afterQuote then
                quoted <- true
            elif ch = '\n' || ch = '\r' then
                doneRecord <- true
            else
                field.Append(ch) |> ignore

            index <- index + 1

        if quoted then
            raise (InvalidDataException("unclosed quoted field in header"))

        fields.Add(field.ToString().Trim())
        fields |> Seq.filter (String.IsNullOrEmpty >> not) |> Seq.toArray

type CsvDataSource(directory: string) =
    member _.Directory = directory

    new(directory: DirectoryInfo) = CsvDataSource(directory.FullName)

    member private this.CsvPath(tableName: string) =
        Path.Combine(this.Directory, tableName + ".csv")

    member private this.ReadTable(tableName: string) =
        try
            File.ReadAllText(this.CsvPath tableName)
        with
        | :? FileNotFoundException as ex -> raise (SqlExecutionException("table not found: " + tableName, ex))
        | :? DirectoryNotFoundException as ex -> raise (SqlExecutionException("table not found: " + tableName, ex))
        | :? IOException as ex -> raise (SqlExecutionException("reading CSV table: " + tableName, ex))

    member this.Schema(tableName: string) =
        (this :> IDataSource).Schema(tableName)

    member this.Scan(tableName: string) =
        (this :> IDataSource).Scan(tableName)

    static member Coerce(value: string) : obj =
        if value = String.Empty then
            null
        else
            match value.ToLowerInvariant() with
            | "true" -> box true
            | "false" -> box false
            | _ ->
                let mutable integer = 0L
                if Int64.TryParse(value, NumberStyles.Integer, CultureInfo.InvariantCulture, &integer) then
                    box integer
                else
                    let mutable real = 0.0
                    if Double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, &real) then
                        box real
                    else
                        box value

    interface IDataSource with
        member this.Schema(tableName: string) =
            let source = this.ReadTable tableName
            try
                CsvHeader.parse source
            with ex ->
                raise (SqlExecutionException("parsing CSV header for table " + tableName, ex))

        member this.Scan(tableName: string) =
            let source = this.ReadTable tableName
            try
                CsvParser.parseCsv source
                |> Array.map (fun row ->
                    let values = Dictionary<string, obj>(StringComparer.Ordinal)
                    for entry in row do
                        values[entry.Key] <- CsvDataSource.Coerce(entry.Value)
                    values :> IReadOnlyDictionary<string, obj>)
            with ex ->
                raise (SqlExecutionException("parsing CSV for table " + tableName, ex))

[<RequireQualifiedAccess>]
module SqlCsvSource =
    let csvDataSource (directory: string) =
        CsvDataSource(directory)

    let executeCsv (sql: string) (directory: string) =
        SqlExecutionEngine.execute sql (CsvDataSource(directory))

    let tryExecuteCsv (sql: string) (directory: string) =
        SqlExecutionEngine.tryExecute sql (CsvDataSource(directory))
