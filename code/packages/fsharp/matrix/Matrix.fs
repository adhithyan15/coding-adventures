namespace CodingAdventures.Matrix.FSharp

open System

[<RequireQualifiedAccess>]
module private MatrixData =
    let validateAndCopy (data: float array array) =
        if isNull data then nullArg "data"
        if data.Length = 0 then invalidArg "data" "Matrix must have at least one row and column"
        if isNull data.[0] || data.[0].Length = 0 then invalidArg "data" "Matrix must have at least one row and column"

        let cols = data.[0].Length
        data
        |> Array.mapi (fun row values ->
            if isNull values then nullArg $"data[{row}]"
            if values.Length <> cols then
                invalidArg "data" $"Row {row} has {values.Length} columns, expected {cols}"
            Array.copy values)

[<AllowNullLiteral>]
type Matrix(input: float array array) =
    let data = MatrixData.validateAndCopy input

    member _.Rows = data.Length
    member _.Cols = data.[0].Length

    member _.Get(row: int, col: int) = data.[row].[col]

    member this.Item
        with get (row: int, col: int) = this.Get(row, col)

    member _.GetData() =
        data |> Array.map Array.copy

    member this.Add(other: Matrix) =
        this.CheckDimensions(other, "add")
        this.Map2(other, fun left right -> left + right)

    member this.Subtract(other: Matrix) =
        this.CheckDimensions(other, "subtract")
        this.Map2(other, fun left right -> left - right)

    member this.AddScalar(scalar: float) =
        this.Map(fun value -> value + scalar)

    member this.SubtractScalar(scalar: float) =
        this.AddScalar(-scalar)

    member this.Scale(scalar: float) =
        this.Map(fun value -> value * scalar)

    member this.Transpose() =
        Array.init this.Cols (fun col ->
            Array.init this.Rows (fun row -> data.[row].[col]))
        |> Matrix

    member this.Dot(other: Matrix) =
        if isNull other then nullArg "other"

        if this.Cols <> other.Rows then
            invalidArg "other" $"Dot dimension mismatch: {this.Rows}x{this.Cols} dot {other.Rows}x{other.Cols}"

        Array.init this.Rows (fun row ->
            Array.init other.Cols (fun col ->
                seq { 0 .. this.Cols - 1 }
                |> Seq.sumBy (fun k -> data.[row].[k] * other.Get(k, col))))
        |> Matrix

    member private this.CheckDimensions(other: Matrix, operation: string) =
        if isNull other then nullArg "other"

        if this.Rows <> other.Rows || this.Cols <> other.Cols then
            invalidArg "other" $"{operation} dimension mismatch: {this.Rows}x{this.Cols} vs {other.Rows}x{other.Cols}"

    member private this.Map(mapper: float -> float) =
        Array.init this.Rows (fun row ->
            Array.init this.Cols (fun col -> mapper data.[row].[col]))
        |> Matrix

    member private this.Map2(other: Matrix, mapper: float -> float -> float) =
        Array.init this.Rows (fun row ->
            Array.init this.Cols (fun col -> mapper data.[row].[col] (other.Get(row, col))))
        |> Matrix

    override this.Equals(obj: obj) =
        match obj with
        | :? Matrix as other when this.Rows = other.Rows && this.Cols = other.Cols ->
            seq {
                for row in 0 .. this.Rows - 1 do
                    for col in 0 .. this.Cols - 1 do
                        data.[row].[col] = other.Get(row, col)
            }
            |> Seq.forall id
        | _ -> false

    override _.GetHashCode() =
        let mutable hash = HashCode()
        hash.Add(data.Length)
        hash.Add(data.[0].Length)
        for row in data do
            for value in row do
                hash.Add(value)
        hash.ToHashCode()

    override this.ToString() =
        $"Matrix({this.Rows}x{this.Cols})"

    static member FromScalar(value: float) =
        Matrix [| [| value |] |]

    static member FromArray(values: float array) =
        if isNull values then nullArg "values"
        if values.Length = 0 then invalidArg "values" "Array must not be empty"
        Matrix [| Array.copy values |]

    static member Zeros(rows: int, cols: int) =
        if rows <= 0 || cols <= 0 then invalidArg "rows" "Dimensions must be positive"
        Matrix(Array.init rows (fun _ -> Array.zeroCreate cols))
