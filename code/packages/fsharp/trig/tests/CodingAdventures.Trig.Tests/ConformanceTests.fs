namespace CodingAdventures.Trig.Tests

// Bit-exact conformance of CodingAdventures.Trig against the shared TRIG-REF-1
// vectors (PHY00a section 6, code/specs/fixtures/trig-reference-v1).
//
// Every vector is one tab-separated row
//
//     op <TAB> x <TAB> y <TAB> cr <TAB> flags
//
// where x and y are IEEE binary64 *bit patterns* (16 lowercase hex digits), never
// decimals.  We decode x with a bit cast, call the public function, encode the
// result back to bits and compare the two 64-bit integers.  Printed floats are
// never compared: they hide the last digits.  The one relaxation is NaN: a
// reference result of 7ff8000000000000 matches any NaN (VER04p "same").

open System
open System.IO
open Xunit
open CodingAdventures.Trig

module Fixture =
    /// Walk up from the test binary until the shared fixture directory appears.
    let directory =
        let rec climb (dir: DirectoryInfo) =
            if isNull dir then
                failwith "could not find code/specs/fixtures/trig-reference-v1 above the test binary"
            else
                let candidate = Path.Combine(dir.FullName, "code", "specs", "fixtures", "trig-reference-v1")
                if Directory.Exists candidate then candidate else climb dir.Parent

        climb (DirectoryInfo AppContext.BaseDirectory)

    /// The four groups and their row counts, as recorded in manifest.json.
    let groups =
        [ "edge", 2854
          "sweep-binade", 19520
          "sweep-cw", 19300
          "sweep-prng", 20480 ]

    type Row =
        { Op: string
          X: string
          Y: string
          Cr: string
          Flags: string }

    let parseRow (line: string) =
        match line.Split('\t') with
        | [| op; x; y; cr; flags |] ->
            { Op = op
              X = x
              Y = y
              Cr = cr
              Flags = flags }
        | fields -> failwithf "malformed vector row (%d fields): %s" fields.Length line

    let load (name: string) =
        File.ReadAllLines(Path.Combine(directory, name + ".tsv"))
        |> Array.filter (fun line -> line <> "")
        |> Array.map parseRow

    /// Sixteen lowercase hex digits to a double's bits.  Anything else is a corpus error.
    let hexBits (field: string) : int64 =
        let isHex c = (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')

        if field.Length = 16 && Seq.forall isHex field then
            Convert.ToInt64(field, 16)
        else
            failwithf "not 16 lowercase hex digits: %s" field

    let toDouble (field: string) = BitConverter.Int64BitsToDouble(hexBits field)
    let bitsOf (v: float) = BitConverter.DoubleToInt64Bits v
    let hex (v: float) = (bitsOf v).ToString("x16")

    let qnanBits = 0x7FF8000000000000L

module Dispatch =
    open Fixture

    /// Call the port's public function for one row.  None stands for the port's
    /// own error (an ArgumentOutOfRangeException from sqrt of a negative).
    let call (op: string) (x: string) : float option =
        let arg () = toDouble x

        match op with
        | "sin" -> Some(Trig.sin (arg ()))
        | "cos" -> Some(Trig.cos (arg ()))
        | "tan" -> Some(Trig.tan (arg ()))
        | "radians" -> Some(Trig.radians (arg ()))
        | "degrees" -> Some(Trig.degrees (arg ()))
        | "atan" -> Some(Trig.atan (arg ()))
        | "sqrt" ->
            try
                Some(Trig.sqrt (arg ()))
            with :? ArgumentOutOfRangeException ->
                None
        | "atan2" ->
            let parts = x.Split(',')
            Some(Trig.atan2 (toDouble parts[0]) (toDouble parts[1]))
        | _ -> failwithf "unknown op in the corpus: %s" op

    /// Why a row failed, or None if the port matches the reference bit for bit.
    let check (row: Row) : string option =
        let got = call row.Op row.X

        if row.Y = "error" then
            match got with
            | None -> None
            | Some v -> Some(sprintf "expected an error, got %s" (hex v))
        else
            match got with
            | None -> Some "unexpected error"
            | Some v ->
                let want = hexBits row.Y

                if want = qnanBits then
                    if Double.IsNaN v then None else Some(sprintf "expected NaN, got %s" (hex v))
                elif bitsOf v = want then
                    None
                else
                    Some(sprintf "expected %s, got %s" row.Y (hex v))

module Accuracy =
    open Fixture

    /// Position of a double on the number line as a signed integer: adjacent
    /// doubles differ by one, and -0 and +0 coincide.
    let ordered (bits: int64) : int64 =
        if bits < 0L then -(bits &&& Int64.MaxValue) else bits

    /// Accuracy limit in steps between adjacent doubles, against the correctly
    /// rounded column.  Some None = named as a no-claim op (tan: PHY00a 3.8).
    /// None = no limit stated, which fails closed.
    let limit (op: string) : int64 option option =
        match op with
        | "sin"
        | "cos" -> Some(Some 3L)
        | "radians"
        | "degrees" -> Some(Some 1L)
        | "tan" -> Some None
        | _ -> None

type ConformanceTests() =
    [<Fact>]
    member _.``Every row of every group matches the reference bit for bit``() =
        let report = Text.StringBuilder()
        let mutable total = 0

        for (name, count) in Fixture.groups do
            let rows = Fixture.load name
            Assert.Equal(count, rows.Length)

            let failures =
                rows
                |> Array.choose (fun r -> Dispatch.check r |> Option.map (fun why -> r, why))

            total <- total + failures.Length

            if failures.Length > 0 then
                report.AppendLine(sprintf "%s: %d of %d rows differ" name failures.Length count) |> ignore

                for (op, n) in failures |> Array.countBy (fun (r, _) -> r.Op.ToString()) |> Array.sortBy fst do
                    report.AppendLine(sprintf "  %s: %d failing rows" op n) |> ignore

                for (r, why) in Array.truncate 5 failures do
                    report.AppendLine(sprintf "    %s %s: %s" r.Op r.X why) |> ignore

        Assert.True((total = 0), report.ToString())

    [<Fact>]
    member _.``Accuracy against the correctly rounded column``() =
        // sin and cos within 3 steps, radians and degrees within 1, tan is the one
        // named no-claim op.  Fail closed: any other op that carries a cr value
        // has no limit and fails, and so does an unknown op.
        let rows = Fixture.groups |> List.toArray |> Array.collect (fun (n, _) -> Fixture.load n)
        let withCr = rows |> Array.filter (fun r -> r.Cr <> "-")

        let unlimited =
            withCr
            |> Array.filter (fun r -> (Accuracy.limit r.Op).IsNone)
            |> Array.map (fun r -> r.Op)
            |> Array.distinct

        Assert.Empty(unlimited)

        let worst = Collections.Generic.Dictionary<string, int64>()

        for r in withCr do
            match Accuracy.limit r.Op with
            | Some(Some _) ->
                match Dispatch.call r.Op r.X with
                | Some got ->
                    let steps =
                        abs (Accuracy.ordered (Fixture.bitsOf got) - Accuracy.ordered (Fixture.hexBits r.Cr))

                    let prev = if worst.ContainsKey r.Op then worst[r.Op] else 0L
                    worst[r.Op] <- max prev steps
                | None -> failwithf "unexpected error on %s %s" r.Op r.X
            | _ -> ()

        for kv in worst do
            match Accuracy.limit kv.Key with
            | Some(Some bound) -> Assert.True((kv.Value <= bound), sprintf "%s: worst %d steps > %d" kv.Key kv.Value bound)
            | _ -> ()

        // The gate really measured something for each claimed op.
        Assert.Equal<string list>([ "cos"; "degrees"; "radians"; "sin" ], worst.Keys |> Seq.sort |> List.ofSeq)

    [<Fact>]
    member _.``sin of minus zero is minus zero and tiny arguments are returned unchanged``() =
        Assert.Equal(0x8000000000000000L, Fixture.bitsOf (Trig.sin -0.0))
        Assert.Equal(0L, Fixture.bitsOf (Trig.sin 0.0))
        Assert.Equal(1.0, Trig.cos 0.0)
        Assert.Equal(1.0, Trig.cos -0.0)
        let tiny = BitConverter.Int64BitsToDouble 0x3E3FFFFFFFFFFFFFL
        Assert.Equal(Fixture.bitsOf tiny, Fixture.bitsOf (Trig.sin tiny))
        Assert.Equal(Double.Epsilon, Trig.sin Double.Epsilon)
        Assert.Equal(1.0, Trig.cos tiny)

    [<Fact>]
    member _.``sin cos and tan are NaN outside the domain``() =
        let beyond = Math.ScaleB(1.0, 29) * (1.0 + Math.ScaleB(1.0, -52))

        for x in [ Double.NaN; Double.PositiveInfinity; Double.NegativeInfinity; 1e20; beyond; -beyond; Double.MaxValue ] do
            Assert.True(Double.IsNaN(Trig.sin x))
            Assert.True(Double.IsNaN(Trig.cos x))
            Assert.True(Double.IsNaN(Trig.tan x))

        // The last in-domain value is finite.
        Assert.False(Double.IsNaN(Trig.sin (Math.ScaleB(1.0, 29))))
        Assert.False(Double.IsNaN(Trig.cos (Math.ScaleB(1.0, 29))))

    [<Fact>]
    member _.``radians and degrees are single multiplies``() =
        Assert.Equal(Trig.PI, Trig.radians 180.0)
        Assert.Equal(180.0, Trig.degrees Trig.PI)
        Assert.True(Double.IsPositiveInfinity(Trig.degrees 1.7e308))
        Assert.True(Double.IsNaN(Trig.radians Double.NaN))
        Assert.Equal(0x8000000000000000L, Fixture.bitsOf (Trig.radians -0.0))
        Assert.Equal(0x8000000000000000L, Fixture.bitsOf (Trig.degrees -0.0))

    [<Fact>]
    member _.``tan keeps the pole sentinel``() =
        Assert.Equal(1.0e308, Trig.tan (Trig.PI / 2.0))
        Assert.Equal(-1.0e308, Trig.tan (-(Trig.PI / 2.0)))

    [<Fact>]
    member _.``atan2 of a NaN argument is NaN and the pinned conventions hold``() =
        let nan = Double.NaN

        for (y, x) in [ nan, 1.0; 1.0, nan; -1.0, nan; nan, nan; nan, -1.0; 0.0, nan ] do
            Assert.True(Double.IsNaN(Trig.atan2 y x))

        Assert.Equal(Trig.PI, Trig.atan2 -0.0 -1.0)
        Assert.Equal(0.0, Trig.atan2 0.0 0.0)
        Assert.True(Double.IsNaN(Trig.atan2 Double.PositiveInfinity Double.PositiveInfinity))
        Assert.Equal(Trig.PI / 2.0, Trig.atan2 1e300 1e-300)

    [<Fact>]
    member _.``Negative quadrants use the floor modulus``() =
        // k = -1 and k = -5 both land in quadrant 3.
        Assert.Equal(-(Trig.sin (Trig.PI / 2.0)), Trig.sin (-(Trig.PI / 2.0)))
        Assert.True(abs (Trig.cos (-5.0 * (Trig.PI / 2.0))) < 1e-15)
