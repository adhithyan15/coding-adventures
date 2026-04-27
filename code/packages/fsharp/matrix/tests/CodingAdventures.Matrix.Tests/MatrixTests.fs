namespace CodingAdventures.Matrix.FSharp.Tests

open System
open CodingAdventures.Matrix.FSharp
open Xunit

type MatrixTests() =
    [<Fact>]
    member _.``from scalar creates one by one matrix``() =
        let matrix = Matrix.FromScalar 5.0

        Assert.Equal(1, matrix.Rows)
        Assert.Equal(1, matrix.Cols)
        Assert.Equal(5.0, matrix.[0, 0])

    [<Fact>]
    member _.``from array creates row vector``() =
        let matrix = Matrix.FromArray [| 1.0; 2.0; 3.0 |]

        Assert.Equal(1, matrix.Rows)
        Assert.Equal(3, matrix.Cols)
        Assert.Equal(2.0, matrix.Get(0, 1))

    [<Fact>]
    member _.``constructor deep copies rectangular data``() =
        let source = [| [| 1.0; 2.0 |]; [| 3.0; 4.0 |] |]
        let matrix = Matrix source
        source.[0].[0] <- 99.0

        Assert.Equal(2, matrix.Rows)
        Assert.Equal(2, matrix.Cols)
        Assert.Equal(1.0, matrix.[0, 0])

    [<Fact>]
    member _.``zeros creates zero filled matrix``() =
        let matrix = Matrix.Zeros(3, 2)

        Assert.Equal(3, matrix.Rows)
        Assert.Equal(2, matrix.Cols)
        for row in 0 .. matrix.Rows - 1 do
            for col in 0 .. matrix.Cols - 1 do
                Assert.Equal(0.0, matrix.[row, col])

    [<Fact>]
    member _.``invalid construction throws``() =
        Assert.Throws<ArgumentException>(fun () -> Matrix [||] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Matrix [| [||] |] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Matrix [| [| 1.0; 2.0 |]; [| 3.0 |] |] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Matrix.FromArray [||] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Matrix.Zeros(0, 2) |> ignore) |> ignore

    [<Fact>]
    member _.``add adds matrices element wise``() =
        let a = Matrix [| [| 1.0; 2.0 |]; [| 3.0; 4.0 |] |]
        let b = Matrix [| [| 5.0; 6.0 |]; [| 7.0; 8.0 |] |]

        Assert.Equal(Matrix [| [| 6.0; 8.0 |]; [| 10.0; 12.0 |] |], a.Add b)

    [<Fact>]
    member _.``subtract subtracts matrices element wise``() =
        let a = Matrix [| [| 5.0; 6.0 |]; [| 7.0; 8.0 |] |]
        let b = Matrix [| [| 1.0; 2.0 |]; [| 3.0; 4.0 |] |]

        Assert.Equal(Matrix [| [| 4.0; 4.0 |]; [| 4.0; 4.0 |] |], a.Subtract b)

    [<Fact>]
    member _.``scalar operations map every element``() =
        let matrix = Matrix [| [| 1.0; 2.0 |]; [| 3.0; 4.0 |] |]

        Assert.Equal(Matrix [| [| 11.0; 12.0 |]; [| 13.0; 14.0 |] |], matrix.AddScalar 10.0)
        Assert.Equal(Matrix [| [| -4.0; -3.0 |]; [| -2.0; -1.0 |] |], matrix.SubtractScalar 5.0)
        Assert.Equal(Matrix [| [| 2.0; 4.0 |]; [| 6.0; 8.0 |] |], matrix.Scale 2.0)
        Assert.Equal(Matrix.Zeros(2, 2), matrix.Scale 0.0)

    [<Fact>]
    member _.``dimension mismatch throws for element wise operations``() =
        let row = Matrix [| [| 1.0; 2.0 |] |]
        let column = Matrix [| [| 1.0 |]; [| 2.0 |] |]

        Assert.Throws<ArgumentException>(fun () -> row.Add column |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> row.Subtract column |> ignore) |> ignore

    [<Fact>]
    member _.``transpose swaps rows and columns``() =
        let matrix = Matrix [| [| 1.0; 2.0; 3.0 |]; [| 4.0; 5.0; 6.0 |] |]
        let transposed = matrix.Transpose()

        Assert.Equal(3, transposed.Rows)
        Assert.Equal(2, transposed.Cols)
        Assert.Equal(Matrix [| [| 1.0; 4.0 |]; [| 2.0; 5.0 |]; [| 3.0; 6.0 |] |], transposed)
        Assert.Equal(matrix, transposed.Transpose())

    [<Fact>]
    member _.``dot multiplies matrices``() =
        let a = Matrix [| [| 1.0; 2.0 |]; [| 3.0; 4.0 |] |]
        let b = Matrix [| [| 5.0; 6.0 |]; [| 7.0; 8.0 |] |]

        Assert.Equal(Matrix [| [| 19.0; 22.0 |]; [| 43.0; 50.0 |] |], a.Dot b)

    [<Fact>]
    member _.``dot handles non square and identity matrices``() =
        let row = Matrix [| [| 1.0; 2.0; 3.0 |] |]
        let column = Matrix [| [| 4.0 |]; [| 5.0 |]; [| 6.0 |] |]
        let identity = Matrix [| [| 1.0; 0.0 |]; [| 0.0; 1.0 |] |]
        let matrix = Matrix [| [| 1.0; 2.0 |]; [| 3.0; 4.0 |] |]

        Assert.Equal(Matrix.FromScalar 32.0, row.Dot column)
        Assert.Equal(matrix, matrix.Dot identity)
        Assert.Equal(matrix, identity.Dot matrix)

    [<Fact>]
    member _.``dot rejects dimension mismatch``() =
        let matrix = Matrix [| [| 1.0; 2.0 |] |]
        Assert.Throws<ArgumentException>(fun () -> matrix.Dot matrix |> ignore) |> ignore

    [<Fact>]
    member _.``equality hash code and string use matrix values``() =
        let a = Matrix [| [| 1.0; 2.0 |]; [| 3.0; 4.0 |] |]
        let b = Matrix [| [| 1.0; 2.0 |]; [| 3.0; 4.0 |] |]
        let c = Matrix [| [| 1.0; 3.0 |]; [| 3.0; 4.0 |] |]

        Assert.Equal(a, b)
        Assert.True(a.Equals(b :> obj))
        Assert.Equal(a.GetHashCode(), b.GetHashCode())
        Assert.NotEqual(a, c)
        Assert.NotEqual(a, Matrix.FromArray [| 1.0; 2.0 |])
        Assert.Equal("Matrix(2x2)", a.ToString())

    [<Fact>]
    member _.``get data returns deep copy``() =
        let matrix = Matrix [| [| 1.0; 2.0 |] |]
        let copy = matrix.GetData()
        copy.[0].[0] <- 999.0

        Assert.Equal(1.0, matrix.[0, 0])
