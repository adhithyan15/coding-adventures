namespace CodingAdventures.Zeroize.Tests

open System
open Xunit
open CodingAdventures.Zeroize.FSharp

module ZeroizeTests =
    [<Fact>]
    let ``zeroize bytes clears buffer`` () =
        let secret = [| 1uy; 2uy; 3uy; 4uy |]

        Zeroize.zeroizeBytes secret

        Assert.All<byte>(secret, fun value -> Assert.Equal(0uy, value))

    [<Fact>]
    let ``zeroize chars and arrays clear buffers`` () =
        let chars = "secret".ToCharArray()
        let numbers = [| 1; 2; 3 |]

        Zeroize.zeroizeChars chars
        Zeroize.zeroizeArray numbers

        Assert.All<char>(chars, fun value -> Assert.Equal('\000', value))
        Assert.All<int>(numbers, fun value -> Assert.Equal(0, value))

    [<Fact>]
    let ``disposable buffer zeroizes on dispose and is idempotent`` () =
        let secret = [| 9uy; 8uy; 7uy |]
        let wrapper = new ZeroizingBuffer(secret)
        let disposable = wrapper :> IDisposable

        disposable.Dispose()
        disposable.Dispose()

        Assert.Same(secret, wrapper.Buffer)
        Assert.All<byte>(secret, fun value -> Assert.Equal(0uy, value))

    [<Fact>]
    let ``validation rejects null`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Zeroize.zeroizeBytes null) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Zeroize.zeroizeChars null) |> ignore
        let nullInts: int array = null
        Assert.Throws<ArgumentNullException>(fun () -> Zeroize.zeroizeArray nullInts) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> new ZeroizingBuffer(null) |> ignore) |> ignore
