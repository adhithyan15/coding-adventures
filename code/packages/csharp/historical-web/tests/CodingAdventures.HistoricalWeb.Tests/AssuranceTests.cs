using System.Collections;
using System.Text;
using System.Text.Json;
using CodingAdventures.HistoricalWeb;
using CodingAdventures.TestBootstrap.Fuzz;
using CodingAdventures.TestBootstrap.Pbt;

namespace CodingAdventures.HistoricalWeb.Tests;

public sealed class AssuranceTests
{
    private const ulong Seed = 0x485745423130UL;

    private static string ReadFixture()
    {
        DirectoryInfo? dir = new(AppContext.BaseDirectory);
        while (dir != null)
        {
            string path = Path.Combine(dir.FullName, "code", "specs", "fixtures", "historical-web-v1", "vectors.json");
            if (File.Exists(path))
            {
                return File.ReadAllText(path, new UTF8Encoding(false, true));
            }

            dir = dir.Parent;
        }

        throw new FileNotFoundException("Could not locate historical-web-v1/vectors.json from test output directory.");
    }

    private static byte[] Hex(string value)
    {
        if ((value.Length & 1) != 0)
        {
            throw new InvalidDataException("Odd-length hexadecimal fixture value.");
        }

        try
        {
            return Convert.FromHexString(value);
        }
        catch (FormatException error)
        {
            throw new InvalidDataException("Invalid hexadecimal fixture value.", error);
        }
    }

    private static JsonElement FixtureRoot(JsonDocument document) => document.RootElement;

    private static bool ValidAddress(ReadOnlySpan<byte> address)
    {
        if (address.IsEmpty)
        {
            return false;
        }

        foreach (byte value in address)
        {
            if (value is < 0x21 or > 0x7e)
            {
                return false;
            }
        }

        return true;
    }

    private static bool ExpectedRequest(ReadOnlySpan<byte> request, out byte[] address)
    {
        address = Array.Empty<byte>();
        ReadOnlySpan<byte> prefix = "GET "u8;
        if (request.Length < prefix.Length || !request[..prefix.Length].SequenceEqual(prefix))
        {
            return false;
        }

        int end = request.Length;
        if (end >= 2 && request[^2] == (byte)'\r' && request[^1] == (byte)'\n')
        {
            end -= 2;
        }
        else if (end >= 1 && request[^1] == (byte)'\n')
        {
            end--;
        }
        else
        {
            return false;
        }

        ReadOnlySpan<byte> candidate = request[prefix.Length..end];
        if (!ValidAddress(candidate))
        {
            return false;
        }

        address = candidate.ToArray();
        return true;
    }

    private static void AssertNode(HtmlDocument document, int index, JsonElement expected)
    {
        HtmlNode actual = document.NodeAt(index);
        if (expected.ValueKind == JsonValueKind.String)
        {
            Assert.Equal(HtmlNodeKind.Text, actual.Kind);
            Assert.Equal(expected.GetString(), actual.Text);
            return;
        }

        Assert.Equal(JsonValueKind.Object, expected.ValueKind);
        Assert.Equal(HtmlNodeKind.Element, actual.Kind);
        Assert.Equal(expected.GetProperty("tag").GetString(), actual.Tag);

        JsonProperty[] expectedAttributes = expected.GetProperty("attributes").EnumerateObject().ToArray();
        Assert.Equal(expectedAttributes.Length, actual.AttributeCount);
        for (int i = 0; i < expectedAttributes.Length; i++)
        {
            HtmlAttribute attribute = document.AttributeAt(actual.FirstAttributeIndex + i);
            JsonProperty expectedAttribute = expectedAttributes[i];
            Assert.Equal(expectedAttribute.Name, attribute.Name);
            Assert.Equal(
                expectedAttribute.Value.ValueKind == JsonValueKind.Null ? null : expectedAttribute.Value.GetString(),
                attribute.Value);
        }

        int childIndex = actual.FirstChildIndex;
        JsonElement.ArrayEnumerator expectedChildren = expected.GetProperty("children").EnumerateArray();
        int childCount = 0;
        while (expectedChildren.MoveNext())
        {
            Assert.NotEqual(-1, childIndex);
            Assert.InRange(childIndex, 0, document.NodeCount - 1);
            AssertNode(document, childIndex, expectedChildren.Current);
            childIndex = document.NodeAt(childIndex).NextSiblingIndex;
            childCount++;
        }

        if (actual.Kind == HtmlNodeKind.Element)
        {
            Assert.Equal(childCount, expected.GetProperty("children").GetArrayLength());
        }

        Assert.Equal(-1, childIndex);
    }

    private static void AssertTree(HtmlDocument document, JsonElement expectedRoots)
    {
        Assert.Equal(JsonValueKind.Array, expectedRoots.ValueKind);
        Assert.Equal(expectedRoots.GetArrayLength(), document.RootCount);
        int root = document.FirstRoot;
        foreach (JsonElement expected in expectedRoots.EnumerateArray())
        {
            Assert.NotEqual(-1, root);
            Assert.InRange(root, 0, document.NodeCount - 1);
            Assert.Equal(-1, document.NodeAt(root).ParentIndex);
            AssertNode(document, root, expected);
            root = document.NodeAt(root).NextSiblingIndex;
        }

        Assert.Equal(-1, root);
    }

    private static bool ArenaValid(HtmlDocument document)
    {
        if (document.NodeCount != document.Nodes.Count || document.AttributeCount != document.Attributes.Count)
        {
            return false;
        }

        bool[] seen = new bool[document.NodeCount];
        Stack<int> pending = new();
        int roots = 0;
        int root = document.FirstRoot;
        while (root != -1)
        {
            if ((uint)root >= (uint)document.NodeCount || ++roots > document.NodeCount || document.NodeAt(root).ParentIndex != -1)
            {
                return false;
            }

            pending.Push(root);
            root = document.NodeAt(root).NextSiblingIndex;
        }

        if (roots != document.RootCount)
        {
            return false;
        }

        while (pending.Count > 0)
        {
            int index = pending.Pop();
            if ((uint)index >= (uint)document.NodeCount || seen[index])
            {
                return false;
            }

            seen[index] = true;
            HtmlNode node = document.NodeAt(index);
            if (node.ParentIndex != -1 && node.ParentIndex >= index)
            {
                return false;
            }

            if (node.Kind == HtmlNodeKind.Element)
            {
                if (node.AttributeCount == 0)
                {
                    if (node.FirstAttributeIndex != -1 && node.FirstAttributeIndex > document.AttributeCount)
                    {
                        return false;
                    }
                }
                else if (node.FirstAttributeIndex < 0 || node.FirstAttributeIndex > document.AttributeCount ||
                         node.AttributeCount > document.AttributeCount - node.FirstAttributeIndex)
                {
                    return false;
                }
            }
            else if (node.AttributeCount != 0 || node.FirstAttributeIndex != -1)
            {
                return false;
            }

            int child = node.FirstChildIndex;
            int children = 0;
            while (child != -1)
            {
                if ((uint)child >= (uint)document.NodeCount || child <= index || ++children > document.NodeCount ||
                    document.NodeAt(child).ParentIndex != index)
                {
                    return false;
                }

                pending.Push(child);
                int next = document.NodeAt(child).NextSiblingIndex;
                if (next != -1 && next <= child)
                {
                    return false;
                }

                child = next;
            }
        }

        return seen.All(value => value);
    }

    [Fact]
    public void SharedHttpFixturesAreFailClosedAndExact()
    {
        using JsonDocument fixture = JsonDocument.Parse(ReadFixture());
        JsonElement http = FixtureRoot(fixture).GetProperty("http-0.9");
        JsonElement requests = http.GetProperty("requests");
        Assert.Equal(3, requests.GetArrayLength());
        foreach (JsonElement vector in requests.EnumerateArray())
        {
            byte[] address = Encoding.UTF8.GetBytes(vector.GetProperty("address").GetString()!);
            byte[] expectedWire = Hex(vector.GetProperty("wire_hex").GetString()!);
            byte[] encoded = Http09.EncodeRequest(address);
            Assert.Equal(Encoding.ASCII.GetBytes("GET " + Encoding.ASCII.GetString(address) + "\r\n"), encoded);
            Assert.Equal(address, Http09.ParseRequest(expectedWire));
        }

        JsonElement invalidAddresses = http.GetProperty("invalid_addresses");
        Assert.Equal(6, invalidAddresses.GetArrayLength());
        foreach (JsonElement vector in invalidAddresses.EnumerateArray())
        {
            HistoricalWebException error = Assert.Throws<HistoricalWebException>(
                () => Http09.EncodeRequest(Encoding.UTF8.GetBytes(vector.GetString()!)));
            Assert.Equal(HistoricalWebErrorKind.InvalidAddress, error.Kind);
        }

        JsonElement invalidRequests = http.GetProperty("invalid_request_hex");
        Assert.Equal(8, invalidRequests.GetArrayLength());
        foreach (JsonElement vector in invalidRequests.EnumerateArray())
        {
            HistoricalWebException error = Assert.Throws<HistoricalWebException>(
                () => Http09.ParseRequest(Hex(vector.GetString()!)));
            Assert.Equal(HistoricalWebErrorKind.InvalidRequest, error.Kind);
        }

        JsonElement responses = http.GetProperty("responses");
        Assert.Equal(3, responses.GetArrayLength());
        foreach (JsonElement vector in responses.EnumerateArray())
        {
            Assert.Equal(vector.GetProperty("body").GetString(), Http09.DecodeResponse(Hex(vector.GetProperty("wire_hex").GetString()!)));
        }

        JsonElement invalidResponses = http.GetProperty("invalid_response_hex");
        Assert.Equal(2, invalidResponses.GetArrayLength());
        foreach (JsonElement vector in invalidResponses.EnumerateArray())
        {
            HistoricalWebException error = Assert.Throws<HistoricalWebException>(
                () => Http09.DecodeResponse(Hex(vector.GetString()!)));
            Assert.Equal(HistoricalWebErrorKind.InvalidResponse, error.Kind);
        }
    }

    [Fact]
    public void SharedHtmlAndExchangeFixturesMatchTheFlatArena()
    {
        using JsonDocument fixture = JsonDocument.Parse(ReadFixture());
        JsonElement root = FixtureRoot(fixture);
        JsonElement cases = root.GetProperty("html-tags-1992");
        Assert.Equal(13, cases.GetArrayLength());
        foreach (JsonElement vector in cases.EnumerateArray())
        {
            HtmlDocument document = HtmlTags1992.Parse(vector.GetProperty("source").GetString()!);
            AssertTree(document, vector.GetProperty("tree"));
            Assert.True(ArenaValid(document));
        }

        JsonElement exchanges = root.GetProperty("exchanges");
        Assert.Equal(1, exchanges.GetArrayLength());
        foreach (JsonElement vector in exchanges.EnumerateArray())
        {
            byte[] address = Encoding.ASCII.GetBytes(vector.GetProperty("address").GetString()!);
            byte[] request = Hex(vector.GetProperty("request_hex").GetString()!);
            Assert.Equal(Encoding.ASCII.GetBytes("GET " + Encoding.ASCII.GetString(address) + "\r\n"), Http09.EncodeRequest(address));
            Assert.Equal(address, Http09.ParseRequest(request));
            byte[] response = Hex(vector.GetProperty("response_hex").GetString()!);
            string body = Http09.DecodeResponse(response);
            Assert.Equal(Encoding.ASCII.GetString(response), body);
            HtmlDocument document = HtmlTags1992.Parse(body);
            AssertTree(document, vector.GetProperty("tree"));
        }
    }

    [Fact]
    public void ExhaustiveShortRequestPayloadsMatchIndependentByteOracle()
    {
        int tested = 0;
        for (int length = 0; length <= 2; length++)
        {
            int total = length switch { 0 => 1, 1 => 256, _ => 65536 };
            for (int word = 0; word < total; word++)
            {
                byte[] payload = new byte[length];
                int remaining = word;
                for (int i = 0; i < length; i++)
                {
                    payload[i] = (byte)(remaining & 0xff);
                    remaining >>= 8;
                }

                foreach (byte[] ending in new[] { new byte[] { (byte)'\n' }, new byte[] { (byte)'\r', (byte)'\n' } })
                {
                    byte[] prefix = "GET "u8.ToArray();
                    byte[] wire = new byte[prefix.Length + payload.Length + ending.Length];
                    prefix.CopyTo(wire, 0);
                    payload.CopyTo(wire, prefix.Length);
                    ending.CopyTo(wire, prefix.Length + payload.Length);
                    bool accepts = ExpectedRequest(wire, out byte[] expected);
                    try
                    {
                        byte[] actual = Http09.ParseRequest(wire);
                        Assert.True(accepts, Convert.ToHexString(wire));
                        Assert.Equal(expected, actual);
                    }
                    catch (HistoricalWebException error)
                    {
                        Assert.False(accepts, Convert.ToHexString(wire));
                        Assert.Equal(HistoricalWebErrorKind.InvalidRequest, error.Kind);
                    }

                    tested++;
                }
            }
        }

        Assert.Equal(131586, tested);
    }

    [Fact]
    public void DiagnosticsAndByteOwnershipAreStable()
    {
        byte[] address = Encoding.ASCII.GetBytes("/index");
        byte[] encoded = Http09.EncodeRequest(address);
        Array.Fill(address, (byte)'X');
        Assert.Equal(Encoding.ASCII.GetBytes("GET /index\r\n"), encoded);
        Array.Fill(encoded, (byte)'X');
        Assert.Equal(Encoding.ASCII.GetBytes("GET /index\r\n"), Http09.EncodeRequest(Encoding.ASCII.GetBytes("/index")));

        byte[] wire = Encoding.ASCII.GetBytes("GET /owned\r\n");
        byte[] parsed = Http09.ParseRequest(wire);
        Array.Fill(wire, (byte)'X');
        Assert.Equal(Encoding.ASCII.GetBytes("/owned"), parsed);

        HistoricalWebException error = Assert.Throws<HistoricalWebException>(() => Http09.ParseRequest(Encoding.ASCII.GetBytes("GXT /\r\n")));
        Assert.Equal(HistoricalWebErrorKind.InvalidRequest, error.Kind);
        Assert.Equal(1, error.Offset);
        byte[] shortPrefix = { (byte)'G', (byte)'E' };
        error = Assert.Throws<HistoricalWebException>(() => Http09.ParseRequest(shortPrefix));
        Assert.Equal(shortPrefix.Length, error.Offset);
        byte[] missingEnd = Encoding.ASCII.GetBytes("GET /missing");
        error = Assert.Throws<HistoricalWebException>(() => Http09.ParseRequest(missingEnd));
        Assert.Equal(missingEnd.Length, error.Offset);
        error = Assert.Throws<HistoricalWebException>(() => Http09.ParseRequest(Encoding.ASCII.GetBytes("GET \n")));
        Assert.Equal(4, error.Offset);
        error = Assert.Throws<HistoricalWebException>(() => Http09.EncodeRequest(Array.Empty<byte>()));
        Assert.Equal(HistoricalWebErrorKind.InvalidAddress, error.Kind);
        Assert.Null(error.Offset);
        error = Assert.Throws<HistoricalWebException>(() => Http09.EncodeRequest(new byte[] { (byte)'/', 0x80 }));
        Assert.Equal(HistoricalWebErrorKind.InvalidAddress, error.Kind);
        Assert.Equal(1, error.Offset);
        error = Assert.Throws<HistoricalWebException>(() => Http09.DecodeResponse(new byte[] { (byte)'A', 0x80 }));
        Assert.Equal(HistoricalWebErrorKind.InvalidResponse, error.Kind);
        Assert.Equal(1, error.Offset);
        Assert.Equal(string.Empty, Http09.DecodeResponse(ReadOnlySpan<byte>.Empty));
        Assert.Equal("\0A\n", Http09.DecodeResponse(new byte[] { 0, (byte)'A', (byte)'\n' }));
    }

    [Fact]
    public void PropertiesCoverArbitraryAndGuaranteedValidInputs()
    {
        Options options = new(500, 1000, CiMode.Off);
        Verdict arbitrary = Runner.Check(Seed, Gens.Bytes(0, 24), bytes =>
        {
            bool valid = ValidAddress(bytes);
            try
            {
                byte[] encoded = Http09.EncodeRequest(bytes);
                return valid && Http09.ParseRequest(encoded).SequenceEqual(bytes) &&
                       encoded.SequenceEqual(Encoding.ASCII.GetBytes("GET " + Encoding.ASCII.GetString(bytes) + "\r\n"));
            }
            catch (HistoricalWebException error)
            {
                return !valid && error.Kind == HistoricalWebErrorKind.InvalidAddress;
            }
        }, options);
        Assert.True(arbitrary.Passed, arbitrary.Message);
        Assert.Equal(500, arbitrary.CasesRun);

        Gen<byte[]> printable = Gens.Map(Gens.Bytes(1, 24), bytes => bytes.Select(value => (byte)(0x21 + value % 94)).ToArray());
        Verdict roundTrips = Runner.Check(Seed, printable, bytes =>
        {
            byte[] encoded = Http09.EncodeRequest(bytes);
            return encoded.SequenceEqual(Encoding.ASCII.GetBytes("GET " + Encoding.ASCII.GetString(bytes) + "\r\n")) &&
                   Http09.ParseRequest(encoded).SequenceEqual(bytes);
        }, new Options(300, 1000, CiMode.Off));
        Assert.True(roundTrips.Passed, roundTrips.Message);
        Assert.Equal(300, roundTrips.CasesRun);

        Gen<string> htmlSources = Gens.Map(Gens.Bytes(0, 64), Encoding.Latin1.GetString);
        Verdict html = Runner.Check(Seed, htmlSources, source => ArenaValid(HtmlTags1992.Parse(source)),
            new Options(400, 1000, CiMode.Off));
        Assert.True(html.Passed, html.Message);
        Assert.Equal(400, html.CasesRun);
    }

    [Fact]
    public void FuzzCampaignChecksIndependentRequestAndArenaOracles()
    {
        IReadOnlyList<byte[]> seeds = new byte[][]
        {
            Array.Empty<byte>(), Encoding.ASCII.GetBytes("GET /\r\n"), Encoding.ASCII.GetBytes("GET /\n"),
            Encoding.ASCII.GetBytes("GET / HTTP/1.0\r\n"), Encoding.ASCII.GetBytes("<A HREF=/x>link</A>"),
            Encoding.ASCII.GetBytes("<XMP><A>&amp;</XMP><P>after"), new byte[] { 0xff, 0x80, (byte)'<' },
        };
        FuzzOptions options = new(1200, Seed, 96, 8, 100, 500000, 64 << 20);
        FuzzReport report = FuzzEngine.Run(seeds, options, (bytes, handle) =>
        {
            handle.Edge(Handle.EdgeId("input/length" + bytes.Length));
            bool requestAccepted = ExpectedRequest(bytes, out byte[] expectedAddress);
            handle.Edge(Handle.EdgeId(requestAccepted ? "http09/accepted" : "http09/rejected"));
            try
            {
                byte[] actual = Http09.ParseRequest(bytes);
                if (!requestAccepted || !actual.SequenceEqual(expectedAddress))
                {
                    throw new InvalidOperationException("independent HTTP oracle mismatch");
                }
            }
            catch (HistoricalWebException error)
            {
                if (requestAccepted || error.Kind != HistoricalWebErrorKind.InvalidRequest)
                {
                    throw;
                }
            }

            HtmlDocument document = HtmlTags1992.Parse(Encoding.Latin1.GetString(bytes));
            handle.Edge(Handle.EdgeId("html1992/recovered"));
            if (document.RootCount > 0) handle.Edge(Handle.EdgeId("html1992/root"));
            if (document.AttributeCount > 0) handle.Edge(Handle.EdgeId("html1992/attribute"));
            if (document.NodeCount > document.RootCount) handle.Edge(Handle.EdgeId("html1992/child"));
            if (!ArenaValid(document)) throw new InvalidOperationException("arena invariant mismatch");
        });
        Assert.True(report.IsClean);
        Assert.Equal(1200, report.Iterations);
        Assert.Equal(FuzzReport.TierReported, report.CoverageTier);
        Assert.True(report.TotalEdges > 0);
    }

    [Fact]
    public void UnicodeRawTextMalformedValuesAndImmutabilityBoundaries()
    {
        HistoricalWebException high = Assert.Throws<HistoricalWebException>(() => HtmlTags1992.Parse("prefix\ud800"));
        Assert.Equal(HistoricalWebErrorKind.InvalidUnicode, high.Kind);
        Assert.Equal(6, high.Offset);
        HistoricalWebException low = Assert.Throws<HistoricalWebException>(() => HtmlTags1992.Parse("\udc00"));
        Assert.Equal(0, low.Offset);
        Assert.Throws<ArgumentNullException>(() => HtmlTags1992.Parse(null!));

        string validUnicode = "é水🛰\0";
        HtmlDocument unicode = HtmlTags1992.Parse(validUnicode);
        Assert.Equal(validUnicode, unicode.NodeAt(unicode.FirstRoot).Text);
        HtmlDocument empty = HtmlTags1992.Parse(string.Empty);
        Assert.Equal(0, empty.NodeCount);
        Assert.Equal(0, empty.RootCount);
        Assert.Equal(-1, empty.FirstRoot);
        string rawText = "aaaaaé";
        HtmlDocument raw = HtmlTags1992.Parse("<xmp>" + rawText + "</XMP><P>after");
        HtmlNode rawNode = raw.NodeAt(raw.FirstRoot);
        Assert.Equal("xmp", rawNode.Tag);
        Assert.Equal(rawText, raw.NodeAt(rawNode.FirstChildIndex).Text);
        int marker = rawNode.NextSiblingIndex;
        Assert.Equal("p", raw.NodeAt(marker).Tag);
        Assert.Equal("after", raw.NodeAt(raw.NodeAt(marker).NextSiblingIndex).Text);

        HtmlDocument anchors = HtmlTags1992.Parse("<A HREF HREF='' NAME=x NAME=y TYPE>z</A><A HREF=>x</A><A HREF==y>q</A>");
        HtmlNode anchor = anchors.NodeAt(anchors.FirstRoot);
        Assert.Equal(3, anchor.AttributeCount);
        Assert.Null(anchors.AttributeAt(anchor.FirstAttributeIndex).Value);
        Assert.Equal("name", anchors.AttributeAt(anchor.FirstAttributeIndex + 1).Name);
        Assert.Equal("x", anchors.AttributeAt(anchor.FirstAttributeIndex + 1).Value);
        Assert.Null(anchors.AttributeAt(anchor.FirstAttributeIndex + 2).Value);

        HtmlDocument nested = HtmlTags1992.Parse("<ADDRESS><ADDRESS>inner</ADDRESS></ADDRESS></A></A>plain");
        Assert.True(ArenaValid(nested));
        Assert.Equal(2, nested.RootCount);
        Assert.Equal("plain", nested.NodeAt(nested.Nodes.Count - 1).Text);
        Assert.Throws<ArgumentOutOfRangeException>(() => nested.NodeAt(-1));
        Assert.Throws<ArgumentOutOfRangeException>(() => nested.NodeAt(nested.NodeCount));
        Assert.Throws<ArgumentOutOfRangeException>(() => nested.AttributeAt(nested.AttributeCount));

        AssertReadOnlyCollectionBoundary(nested.Nodes);
        AssertReadOnlyCollectionBoundary(nested.Attributes);
    }

    [Fact]
    public void EverySingleUtf16CodeUnitIsEitherPreservedOrRejectedAtZero()
    {
        int accepted = 0;
        int rejected = 0;
        for (int codeUnit = 0; codeUnit <= char.MaxValue; codeUnit++)
        {
            string source = new((char)codeUnit, 1);
            if (char.IsSurrogate(source[0]))
            {
                HistoricalWebException error = Assert.Throws<HistoricalWebException>(() => HtmlTags1992.Parse(source));
                Assert.Equal(HistoricalWebErrorKind.InvalidUnicode, error.Kind);
                Assert.Equal(0, error.Offset);
                rejected++;
            }
            else
            {
                HtmlDocument document = HtmlTags1992.Parse(source);
                Assert.Equal(1, document.RootCount);
                Assert.Equal(HtmlNodeKind.Text, document.NodeAt(document.FirstRoot).Kind);
                Assert.Equal(source, document.NodeAt(document.FirstRoot).Text);
                accepted++;
            }
        }

        Assert.Equal(63488, accepted);
        Assert.Equal(2048, rejected);
    }

    private static void AssertReadOnlyCollectionBoundary<T>(IReadOnlyList<T> view)
    {
        Assert.False(view is T[]);
        Assert.Equal(view.Count, view.ToArray().Length);
        if (view is ICollection<T> genericCollection)
        {
            Assert.True(genericCollection.IsReadOnly);
        }

        if (view is IList<T> typedList)
        {
            Assert.True(typedList.IsReadOnly);
        }

        if (view is IList nonGenericList)
        {
            Assert.True(nonGenericList.IsReadOnly);
        }

        if (view is ICollection collection)
        {
            object syncRoot = collection.SyncRoot;
            Assert.False(syncRoot is T[]);
            if (syncRoot is IList syncList)
            {
                Assert.True(syncList.IsReadOnly);
            }
        }
    }

    [Fact]
    public void DeepArenaConstructionAndDisposalAreIterative()
    {
        const int depth = 12000;
        const int unmatched = 4096;
        StringBuilder source = new(depth * 18 + unmatched * 4);
        for (int i = 0; i < depth; i++) source.Append("<ADDRESS>");
        for (int i = 0; i < unmatched; i++) source.Append("</A>");
        source.Append('x');
        for (int i = 0; i < depth; i++) source.Append("</ADDRESS>");

        HtmlDocument document = HtmlTags1992.Parse(source.ToString());
        Assert.Equal(depth + 1, document.NodeCount);
        Assert.True(ArenaValid(document));
        Assert.Equal("address", document.NodeAt(document.FirstRoot).Tag);
    }
}
