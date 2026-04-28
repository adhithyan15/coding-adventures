using System.Text;

namespace CodingAdventures.IrcProto.Tests;

public sealed class IrcProtoTests
{
    private static Message RoundTrip(string line)
    {
        var first = IrcProto.Parse(line);
        var wire = IrcProto.Serialize(first);
        return IrcProto.Parse(Encoding.UTF8.GetString(wire).TrimEnd('\r', '\n'));
    }

    [Fact]
    public void VersionIsStable()
    {
        Assert.Equal("0.1.0", IrcProto.Version);
    }

    [Theory]
    [InlineData("NICK alice", null, "NICK", new[] { "alice" })]
    [InlineData("join #general", null, "JOIN", new[] { "#general" })]
    [InlineData(":irc.local 001 alice :Welcome!", "irc.local", "001", new[] { "alice", "Welcome!" })]
    [InlineData(":alice!alice@host PRIVMSG #general :hello world", "alice!alice@host", "PRIVMSG", new[] { "#general", "hello world" })]
    [InlineData("PING", null, "PING", new string[] { })]
    public void ParseHappyPaths(string line, string? prefix, string command, string[] parameters)
    {
        var message = IrcProto.Parse(line);

        Assert.Equal(prefix, message.Prefix);
        Assert.Equal(command, message.Command);
        Assert.Equal(parameters, message.Params);
    }

    [Fact]
    public void ParsePreservesTrailingSpacesAndEmptyTrailing()
    {
        Assert.Equal(["#c", "hello   world"], IrcProto.Parse("PRIVMSG #c :hello   world").Params);
        Assert.Equal(["#c", ""], IrcProto.Parse("PRIVMSG #c :").Params);
    }

    [Fact]
    public void ParseCapsParametersAtFifteen()
    {
        var parameters = string.Join(" ", Enumerable.Range(0, 16));

        var message = IrcProto.Parse($"CMD {parameters}");

        Assert.Equal(15, message.Params.Count);
        Assert.Equal("14", message.Params.Last());
    }

    [Theory]
    [InlineData("")]
    [InlineData(" ")]
    [InlineData("   ")]
    [InlineData(":irc.local")]
    [InlineData(":irc.local ")]
    public void ParseRejectsMalformedLines(string line)
    {
        Assert.Throws<ParseError>(() => IrcProto.Parse(line));
    }

    [Fact]
    public void SerializeWritesCrlfAndTrailingParams()
    {
        var message = new Message(null, "PRIVMSG", ["#chan", "hello world"]);

        var wire = IrcProto.Serialize(message);

        Assert.Equal("PRIVMSG #chan :hello world\r\n", Encoding.UTF8.GetString(wire));
        Assert.EndsWith("\r\n", Encoding.UTF8.GetString(wire));
    }

    [Fact]
    public void SerializeHandlesPrefixesParamsAndEmptyLastParam()
    {
        Assert.Equal(
            ":irc.local 001 alice Welcome!\r\n",
            Encoding.UTF8.GetString(IrcProto.Serialize(new Message("irc.local", "001", ["alice", "Welcome!"]))));
        Assert.Equal(
            "PRIVMSG #c \r\n",
            Encoding.UTF8.GetString(IrcProto.Serialize(new Message(null, "PRIVMSG", ["#c", ""]))));
    }

    [Fact]
    public void SerializeUsesUtf8()
    {
        var wire = IrcProto.Serialize(new Message(null, "PRIVMSG", ["#ch", "H\u00e9llo"]));

        Assert.Contains((byte)0xC3, wire);
    }

    [Theory]
    [InlineData("NICK alice", null, "NICK", new[] { "alice" })]
    [InlineData("PRIVMSG #general :Hello everyone!", null, "PRIVMSG", new[] { "#general", "Hello everyone!" })]
    [InlineData(":srv 433 * nick :Nick in use", "srv", "433", new[] { "*", "nick", "Nick in use" })]
    public void RoundTripsCanonicalMessages(string line, string? prefix, string command, string[] parameters)
    {
        var message = RoundTrip(line);

        Assert.Equal(prefix, message.Prefix);
        Assert.Equal(command, message.Command);
        Assert.Equal(parameters, message.Params);
    }
}
