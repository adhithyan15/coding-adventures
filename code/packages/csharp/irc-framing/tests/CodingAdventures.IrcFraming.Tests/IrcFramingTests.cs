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

    // -----------------------------------------------------------------
    // SEC-011: a peer that never sends LF must not grow the buffer.
    //
    // The cap on an unterminated line is 511 bytes: 510 content bytes plus
    // a CR whose LF has not arrived yet (RFC 1459 / RFC 2812 section 2.3:
    // 512 bytes including CRLF).  These tests mirror the Rust crate's.
    // -----------------------------------------------------------------

    private const int SixteenMiB = 16 * 1024 * 1024;

    private static byte[] Filled(int count, char c) => Enumerable.Repeat((byte)c, count).ToArray();

    [Fact]
    public void NoLfInChunksIsBounded()
    {
        var framer = new Framer();
        var chunk = Filled(4096, 'A');
        var worst = 0;
        for (var i = 0; i < SixteenMiB / chunk.Length; i++)
        {
            framer.Feed(chunk);
            worst = Math.Max(worst, framer.BufferSize);
        }

        Assert.True(worst <= 512, $"buffer grew to {worst} bytes with no LF");
        Assert.Empty(framer.Frames());
    }

    [Fact]
    public void NoLfInOneFeedIsBounded()
    {
        var framer = new Framer();
        framer.Feed(Filled(SixteenMiB, 'A'));

        Assert.True(framer.BufferSize <= 512, $"buffer grew to {framer.BufferSize}");
        Assert.Empty(framer.Frames());
    }

    [Fact]
    public void FramerRecoversAfterFlood()
    {
        var framer = new Framer();
        framer.Feed(Filled(4096, 'A'));
        framer.Feed(Filled(4096, 'B'));
        framer.Feed(Bytes("tail\r\nNICK alice\r\n"));

        var frames = framer.Frames();

        Assert.Single(frames);
        Assert.Equal<byte>(Bytes("NICK alice"), frames[0]);
        Assert.Equal(0, framer.BufferSize);
    }

    [Fact]
    public void PendingLineAtLimitIsKept()
    {
        // 510 content bytes plus a CR is a legal line still waiting for its LF.
        var framer = new Framer();
        framer.Feed(Filled(510, 'A').Append((byte)'\r').ToArray());
        Assert.Equal(511, framer.BufferSize);

        framer.Feed(Bytes("\n"));
        var frames = framer.Frames();

        Assert.Single(frames);
        Assert.Equal(510, frames[0].Length);
    }

    [Fact]
    public void PendingLineOverLimitIsDropped()
    {
        var framer = new Framer();
        framer.Feed(Filled(512, 'A'));
        Assert.Equal(0, framer.BufferSize);

        framer.Feed(Bytes("\r\nPING x\r\n"));
        var frames = framer.Frames();

        Assert.Single(frames);
        Assert.Equal<byte>(Bytes("PING x"), frames[0]);
    }

    [Fact]
    public void CompleteLinesBeforeFloodSurvive()
    {
        var framer = new Framer();
        framer.Feed(Bytes("NICK a\r\n").Concat(Filled(2000, 'A')).ToArray());

        var frames = framer.Frames();

        Assert.Single(frames);
        Assert.Equal<byte>(Bytes("NICK a"), frames[0]);
        Assert.Equal(0, framer.BufferSize);
    }

    [Fact]
    public void ResetClearsDiscardState()
    {
        // Reset must end the skip-to-LF state, or the next line is swallowed.
        var framer = new Framer();
        framer.Feed(Filled(600, 'A'));
        framer.Reset();
        framer.Feed(Bytes("NICK a\r\n"));

        var frames = framer.Frames();

        Assert.Single(frames);
        Assert.Equal<byte>(Bytes("NICK a"), frames[0]);
    }

    private static byte[] Bytes(string text) => Encoding.ASCII.GetBytes(text);
}
