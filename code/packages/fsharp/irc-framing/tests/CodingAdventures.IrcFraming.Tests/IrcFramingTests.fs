namespace CodingAdventures.IrcFraming.Tests

open System
open System.Linq
open System.Text
open Xunit
open CodingAdventures.IrcFraming.FSharp

module IrcFramingTests =
    let private bytes (text: string) = Encoding.ASCII.GetBytes text

    [<Fact>]
    let ``version is exposed`` () =
        Assert.Equal("0.1.0", IrcFraming.VERSION)
        Assert.Equal(510, IrcFraming.MAX_CONTENT_BYTES)

    [<Fact>]
    let ``extracts single crlf frame`` () =
        let framer = Framer()
        framer.Feed(bytes "NICK alice\r\n")

        let frames = framer.Frames()

        Assert.Single(frames) |> ignore
        Assert.Equal<byte>(bytes "NICK alice", frames[0])
        Assert.Equal(0, framer.BufferSize)

    [<Fact>]
    let ``accepts lf only frames`` () =
        let framer = Framer()
        framer.Feed(bytes "NICK alice\nUSER bob\n")

        let frames = framer.Frames()

        Assert.Equal(2, frames.Length)
        Assert.Equal<byte>(bytes "NICK alice", frames[0])
        Assert.Equal<byte>(bytes "USER bob", frames[1])

    [<Fact>]
    let ``extracts multiple frames and leaves partial tail`` () =
        let framer = Framer()
        framer.Feed(bytes "A\r\nB\r\nC")

        let frames = framer.Frames()

        Assert.Equal(2, frames.Length)
        Assert.Equal<byte>(bytes "A", frames[0])
        Assert.Equal<byte>(bytes "B", frames[1])
        Assert.Equal(1, framer.BufferSize)

        framer.Feed(bytes "\r\n")
        let completedTail = framer.Frames()
        Assert.Equal<byte>(bytes "C", completedTail[0])

    [<Fact>]
    let ``buffers messages split across feeds`` () =
        let framer = Framer()
        framer.Feed(bytes "NICK al")

        Assert.Empty(framer.Frames())
        Assert.Equal(7, framer.BufferSize)

        framer.Feed(bytes "ice\r\n")

        let frames = framer.Frames()
        Assert.Equal<byte>(bytes "NICK alice", frames[0])
        Assert.Equal(0, framer.BufferSize)

    [<Fact>]
    let ``handles crlf split across feeds`` () =
        let framer = Framer()
        framer.Feed(bytes "NICK alice\r")
        Assert.Empty(framer.Frames())

        framer.Feed(bytes "\n")

        let frames = framer.Frames()
        Assert.Equal<byte>(bytes "NICK alice", frames[0])

    [<Fact>]
    let ``empty feed is noop and empty line yields empty frame`` () =
        let framer = Framer()
        framer.Feed(Array.empty)
        Assert.Equal(0, framer.BufferSize)
        Assert.Empty(framer.Frames())

        framer.Feed(bytes "\r\n")
        let frame = Assert.Single(framer.Frames())
        Assert.Empty(frame)

    [<Fact>]
    let ``rejects null feed`` () =
        let framer = Framer()
        Assert.Throws<ArgumentNullException>(fun () -> framer.Feed(Unchecked.defaultof<byte array>)) |> ignore

    [<Fact>]
    let ``accepts exact maximum and discards overlong lines`` () =
        let exact = Enumerable.Repeat(byte 'A', 510).ToArray()
        let overlong = Enumerable.Repeat(byte 'X', 511).ToArray()
        let framer = Framer()

        framer.Feed exact
        framer.Feed(bytes "\r\n")
        let exactFrames = framer.Frames()
        Assert.Equal(510, exactFrames[0].Length)

        framer.Feed overlong
        framer.Feed(bytes "\r\n")
        Assert.Empty(framer.Frames())

    [<Fact>]
    let ``overlong line does not block following valid frames`` () =
        let framer = Framer()
        framer.Feed(Enumerable.Repeat(byte 'X', 511).ToArray())
        framer.Feed(bytes "\r\nNICK alice\r\nUSER bob\r\n")

        let frames = framer.Frames()

        Assert.Equal(2, frames.Length)
        Assert.Equal<byte>(bytes "NICK alice", frames[0])
        Assert.Equal<byte>(bytes "USER bob", frames[1])

    [<Fact>]
    let ``reset clears buffered data`` () =
        let framer = Framer()
        framer.Feed(bytes "partial")
        Assert.Equal(7, framer.BufferSize)

        framer.Reset()

        Assert.Equal(0, framer.BufferSize)
        framer.Feed(bytes "PING :x\r\n")
        let frames = framer.Frames()
        Assert.Equal<byte>(bytes "PING :x", frames[0])
