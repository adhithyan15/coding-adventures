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

    // -----------------------------------------------------------------
    // SEC-011: a peer that never sends LF must not grow the buffer.
    //
    // The cap on an unterminated line is 511 bytes: 510 content bytes plus
    // a CR whose LF has not arrived yet (RFC 1459 / RFC 2812 section 2.3:
    // 512 bytes including CRLF).  These tests mirror the Rust crate's.
    // -----------------------------------------------------------------

    let private sixteenMiB = 16 * 1024 * 1024

    let private filled (count: int) (c: char) : byte array = Array.create count (byte c)

    [<Fact>]
    let ``no lf in chunks is bounded`` () =
        let framer = Framer()
        let chunk = filled 4096 'A'
        let mutable worst = 0

        for _ in 1 .. sixteenMiB / chunk.Length do
            framer.Feed(chunk)
            worst <- max worst framer.BufferSize

        Assert.True(worst <= 512, sprintf "buffer grew to %d bytes with no LF" worst)
        Assert.Empty(framer.Frames())

    [<Fact>]
    let ``no lf in one feed is bounded`` () =
        let framer = Framer()
        framer.Feed(filled sixteenMiB 'A')

        Assert.True(framer.BufferSize <= 512, sprintf "buffer grew to %d" framer.BufferSize)
        Assert.Empty(framer.Frames())

    [<Fact>]
    let ``framer recovers after flood`` () =
        let framer = Framer()
        framer.Feed(filled 4096 'A')
        framer.Feed(filled 4096 'B')
        framer.Feed(bytes "tail\r\nNICK alice\r\n")

        let frames = framer.Frames()

        Assert.Single(frames) |> ignore
        Assert.Equal<byte>(bytes "NICK alice", frames[0])
        Assert.Equal(0, framer.BufferSize)

    [<Fact>]
    let ``pending line at limit is kept`` () =
        // 510 content bytes plus a CR is a legal line still waiting for its LF.
        let framer = Framer()
        framer.Feed(Array.append (filled 510 'A') [| byte '\r' |])
        Assert.Equal(511, framer.BufferSize)

        framer.Feed(bytes "\n")
        let frames = framer.Frames()

        Assert.Single(frames) |> ignore
        Assert.Equal(510, frames[0].Length)

    [<Fact>]
    let ``pending line over limit is dropped`` () =
        let framer = Framer()
        framer.Feed(filled 512 'A')
        Assert.Equal(0, framer.BufferSize)

        framer.Feed(bytes "\r\nPING x\r\n")
        let frames = framer.Frames()

        Assert.Single(frames) |> ignore
        Assert.Equal<byte>(bytes "PING x", frames[0])

    [<Fact>]
    let ``complete lines before flood survive`` () =
        let framer = Framer()
        framer.Feed(Array.append (bytes "NICK a\r\n") (filled 2000 'A'))

        let frames = framer.Frames()

        Assert.Single(frames) |> ignore
        Assert.Equal<byte>(bytes "NICK a", frames[0])
        Assert.Equal(0, framer.BufferSize)

    [<Fact>]
    let ``reset clears discard state`` () =
        // Reset must end the skip-to-LF state, or the next line is swallowed.
        let framer = Framer()
        framer.Feed(filled 600 'A')
        framer.Reset()
        framer.Feed(bytes "NICK a\r\n")

        let frames = framer.Frames()

        Assert.Single(frames) |> ignore
        Assert.Equal<byte>(bytes "NICK a", frames[0])
