using System.Text;

namespace CodingAdventures.IrcFraming.Tests;

public sealed class IrcFramingTests
{
    [Fact]
    public void VersionIsExposed()
    {
        Assert.Equal("0.1.0", IrcFraming.Version);
        Assert.Equal(510, Framer.MaxContentBytes);
    }

    [Fact]
    public void ExtractsSingleCrlfFrame()
    {
        var framer = new Framer();
        framer.Feed(Bytes("NICK alice\r\n"));

        var frames = framer.Frames();

        Assert.Single(frames);
        Assert.Equal<byte>(Bytes("NICK alice"), frames[0]);
        Assert.Equal(0, framer.BufferSize);
    }

    [Fact]
    public void AcceptsLfOnlyFrames()
    {
        var framer = new Framer();
        framer.Feed(Bytes("NICK alice\nUSER bob\n"));

        var frames = framer.Frames();

        Assert.Equal(2, frames.Count);
        Assert.Equal<byte>(Bytes("NICK alice"), frames[0]);
        Assert.Equal<byte>(Bytes("USER bob"), frames[1]);
    }

    [Fact]
    public void ExtractsMultipleFramesAndLeavesPartialTail()
    {
        var framer = new Framer();
        framer.Feed(Bytes("A\r\nB\r\nC"));

        var frames = framer.Frames();

        Assert.Equal(2, frames.Count);
        Assert.Equal<byte>(Bytes("A"), frames[0]);
        Assert.Equal<byte>(Bytes("B"), frames[1]);
        Assert.Equal(1, framer.BufferSize);

        framer.Feed(Bytes("\r\n"));
        Assert.Equal<byte>(Bytes("C"), framer.Frames()[0]);
    }

    [Fact]
    public void BuffersMessagesSplitAcrossFeeds()
    {
        var framer = new Framer();
        framer.Feed(Bytes("NICK al"));

        Assert.Empty(framer.Frames());
        Assert.Equal(7, framer.BufferSize);

        framer.Feed(Bytes("ice\r\n"));

        Assert.Equal<byte>(Bytes("NICK alice"), framer.Frames()[0]);
        Assert.Equal(0, framer.BufferSize);
    }

    [Fact]
    public void HandlesCrlfSplitAcrossFeeds()
    {
        var framer = new Framer();
        framer.Feed(Bytes("NICK alice\r"));
        Assert.Empty(framer.Frames());

        framer.Feed(Bytes("\n"));

        Assert.Equal<byte>(Bytes("NICK alice"), framer.Frames()[0]);
    }

    [Fact]
    public void EmptyFeedIsNoopAndEmptyLineYieldsEmptyFrame()
    {
        var framer = new Framer();
        framer.Feed([]);
        Assert.Equal(0, framer.BufferSize);
        Assert.Empty(framer.Frames());

        framer.Feed(Bytes("\r\n"));
        var frame = Assert.Single(framer.Frames());
        Assert.Empty(frame);
    }

    [Fact]
    public void RejectsNullFeed()
    {
        var framer = new Framer();
        Assert.Throws<ArgumentNullException>(() => framer.Feed((byte[]?)null!));
    }

    [Fact]
    public void AcceptsExactMaximumAndDiscardsOverlongLines()
    {
        var exact = Enumerable.Repeat((byte)'A', 510).ToArray();
        var overlong = Enumerable.Repeat((byte)'X', 511).ToArray();
        var framer = new Framer();

        framer.Feed(exact);
        framer.Feed(Bytes("\r\n"));
        Assert.Equal(510, framer.Frames()[0].Length);

        framer.Feed(overlong);
        framer.Feed(Bytes("\r\n"));
        Assert.Empty(framer.Frames());
    }

    [Fact]
    public void OverlongLineDoesNotBlockFollowingValidFrames()
    {
        var framer = new Framer();
        framer.Feed(Enumerable.Repeat((byte)'X', 511).ToArray());
        framer.Feed(Bytes("\r\nNICK alice\r\nUSER bob\r\n"));

        var frames = framer.Frames();

        Assert.Equal(2, frames.Count);
        Assert.Equal<byte>(Bytes("NICK alice"), frames[0]);
        Assert.Equal<byte>(Bytes("USER bob"), frames[1]);
    }

    [Fact]
    public void ResetClearsBufferedData()
    {
        var framer = new Framer();
        framer.Feed(Bytes("partial"));
        Assert.Equal(7, framer.BufferSize);

        framer.Reset();

        Assert.Equal(0, framer.BufferSize);
        framer.Feed(Bytes("PING :x\r\n"));
        Assert.Equal<byte>(Bytes("PING :x"), framer.Frames()[0]);
    }

    private static byte[] Bytes(string text) => Encoding.ASCII.GetBytes(text);
}
