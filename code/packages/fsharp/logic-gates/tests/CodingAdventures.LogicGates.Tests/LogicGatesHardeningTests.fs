namespace CodingAdventures.LogicGates.Tests

open System
open System.Globalization
open System.Reflection
open System.Threading
open Xunit
open CodingAdventures.LogicGates

/// LGE-FS-HARDEN: invariant-culture messages, null-safe names, signature
/// surface, checked arithmetic, and Debug/Release independence.
type HardeningTests() =

    /// Run `f` under a culture whose negative sign is "~", then restore.
    let withOddCulture (f: unit -> unit) =
        let saved = Thread.CurrentThread.CurrentCulture
        let savedUi = Thread.CurrentThread.CurrentUICulture
        try
            let c = CultureInfo("en-US").Clone() :?> CultureInfo
            c.NumberFormat.NegativeSign <- "~"
            Thread.CurrentThread.CurrentCulture <- c
            Thread.CurrentThread.CurrentUICulture <- c
            f ()
        finally
            Thread.CurrentThread.CurrentCulture <- saved
            Thread.CurrentThread.CurrentUICulture <- savedUi

    [<Fact>]
    member _.MessageIsInvariantUnderCustomNegativeSign() =
        withOddCulture (fun () ->
            let ex = Assert.Throws<InvalidBitException>(fun () -> LogicGates.andGate -7 1 |> ignore)
            Assert.Contains("got -7.", ex.Message)
            Assert.Contains("Actual value was -7.", ex.Message)
            Assert.DoesNotContain("~", ex.Message)
            Assert.Contains("(Parameter 'a')", ex.Message))

    [<Fact>]
    member _.NaryMessagesIgnoreCulture() =
        withOddCulture (fun () ->
            let ex = Assert.Throws<ArgumentException>(fun () -> LogicGates.andN (List.replicate 65537 1) |> ignore)
            Assert.Contains("at most 65536 inputs", ex.Message)
            let bad = Assert.Throws<InvalidBitException>(fun () -> LogicGates.orN [ 0; 0; -3 ] |> ignore)
            Assert.Equal("inputs[2]", bad.ParamName)
            Assert.Contains("-3", bad.Message)
            Assert.DoesNotContain("~", bad.Message))

    [<Fact>]
    member _.NullParameterNameIsRejectedNotPropagated() =
        let ex =
            Assert.Throws<ArgumentNullException>(fun () ->
                InvalidBitException(Unchecked.defaultof<string>, 2) |> ignore)
        Assert.Equal("parameter", ex.ParamName)

    [<Fact>]
    member _.ExceptionExposesNameAndValue() =
        let ex = InvalidBitException("zz", 9)
        Assert.Equal("zz", ex.Parameter)
        Assert.Equal("zz", ex.ParamName)
        Assert.Equal(9, ex.Value)
        Assert.True(typeof<InvalidBitException>.IsSealed)
        Assert.IsAssignableFrom<ArgumentOutOfRangeException>(ex) |> ignore

    [<Fact>]
    member _.ExtremesNeverOverflow() =
        // The library is compiled --checked+; no caller input may reach an
        // OverflowException, only InvalidBitException.
        for v in [ Int32.MinValue; Int32.MaxValue; -1; 2 ] do
            Assert.Throws<InvalidBitException>(fun () -> LogicGates.notGate v |> ignore) |> ignore
            Assert.Throws<InvalidBitException>(fun () -> LogicGates.nandXor 1 v |> ignore) |> ignore
            Assert.Throws<InvalidBitException>(fun () -> LogicGates.xorN [ 1; v ] |> ignore) |> ignore

    [<Fact>]
    member _.PublicSurfaceIsMonomorphicInt() =
        let m = typeof<InvalidBitException>.Assembly.GetType("CodingAdventures.LogicGates.LogicGates")
        Assert.NotNull(m)
        let methods = m.GetMethods(BindingFlags.Public ||| BindingFlags.Static)
        Assert.NotEmpty(methods)
        for mi in methods do
            Assert.False(mi.IsGenericMethod, mi.Name)
            Assert.Equal(typeof<int>, mi.ReturnType)
            for p in mi.GetParameters() do
                Assert.True(p.ParameterType = typeof<int> || p.ParameterType = typeof<int list>, mi.Name)
        Assert.Equal(65536, LogicGates.maxInputs)
