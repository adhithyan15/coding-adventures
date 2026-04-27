namespace CodingAdventures.IrcProto.Tests

open System.Text
open Xunit
open CodingAdventures.IrcProto.FSharp

module IrcProtoTests =
    let private roundTrip line =
        let first = IrcProto.parse line
        let wire = IrcProto.serialize first
        Encoding.UTF8.GetString(wire).TrimEnd('\r', '\n') |> IrcProto.parse

    [<Fact>]
    let ``version is stable`` () =
        Assert.Equal("0.1.0", IrcProto.Version)

    [<Theory>]
    [<InlineData("NICK alice", null, "NICK")>]
    [<InlineData("join #general", null, "JOIN")>]
    [<InlineData(":irc.local 001 alice :Welcome!", "irc.local", "001")>]
    [<InlineData(":alice!alice@host PRIVMSG #general :hello world", "alice!alice@host", "PRIVMSG")>]
    [<InlineData("PING", null, "PING")>]
    let ``parse happy paths`` line prefix command =
        let message = IrcProto.parse line

        Assert.Equal((if isNull prefix then None else Some prefix), message.Prefix)
        Assert.Equal(command, message.Command)

    [<Fact>]
    let ``parse parameters and trailing values`` () =
        Assert.Equal<string list>([ "#c"; "hello   world" ], (IrcProto.parse "PRIVMSG #c :hello   world").Params)
        Assert.Equal<string list>([ "#c"; "" ], (IrcProto.parse "PRIVMSG #c :").Params)
        Assert.Equal<string list>([ "alice"; "0"; "*"; "Alice Smith" ], (IrcProto.parse "USER alice 0 * :Alice Smith").Params)

    [<Fact>]
    let ``parse caps parameters at fifteen`` () =
        let parameters = [ 0 .. 15 ] |> List.map string |> String.concat " "

        let message = IrcProto.parse $"CMD {parameters}"

        Assert.Equal(15, message.Params.Length)
        Assert.Equal("14", message.Params |> List.last)

    [<Theory>]
    [<InlineData("")>]
    [<InlineData(" ")>]
    [<InlineData("   ")>]
    [<InlineData(":irc.local")>]
    [<InlineData(":irc.local ")>]
    let ``parse rejects malformed lines`` line =
        Assert.Throws<ParseError>(fun () -> IrcProto.parse line |> ignore) |> ignore

    [<Fact>]
    let ``serialize writes crlf and trailing params`` () =
        let wire =
            IrcProto.serialize
                { Prefix = None
                  Command = "PRIVMSG"
                  Params = [ "#chan"; "hello world" ] }
            |> Encoding.UTF8.GetString

        Assert.Equal("PRIVMSG #chan :hello world\r\n", wire)
        Assert.EndsWith("\r\n", wire)

    [<Fact>]
    let ``serialize handles prefixes params and empty last param`` () =
        let welcome =
            IrcProto.serialize
                { Prefix = Some "irc.local"
                  Command = "001"
                  Params = [ "alice"; "Welcome!" ] }
            |> Encoding.UTF8.GetString

        let empty =
            IrcProto.serialize
                { Prefix = None
                  Command = "PRIVMSG"
                  Params = [ "#c"; "" ] }
            |> Encoding.UTF8.GetString

        Assert.Equal(":irc.local 001 alice Welcome!\r\n", welcome)
        Assert.Equal("PRIVMSG #c \r\n", empty)

    [<Fact>]
    let ``serialize uses utf8`` () =
        let wire =
            IrcProto.serialize
                { Prefix = None
                  Command = "PRIVMSG"
                  Params = [ "#ch"; "H\u00e9llo" ] }

        Assert.Contains(0xC3uy, wire)

    [<Fact>]
    let ``round trips canonical messages`` () =
        Assert.Equal({ Prefix = None; Command = "NICK"; Params = [ "alice" ] }, roundTrip "NICK alice")
        Assert.Equal({ Prefix = None; Command = "PRIVMSG"; Params = [ "#general"; "Hello everyone!" ] }, roundTrip "PRIVMSG #general :Hello everyone!")
        Assert.Equal({ Prefix = Some "srv"; Command = "433"; Params = [ "*"; "nick"; "Nick in use" ] }, roundTrip ":srv 433 * nick :Nick in use")
