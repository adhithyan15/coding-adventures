namespace CodingAdventures.HistoricalWeb.FSharp.Tests

open System
open System.IO
open System.Text
open System.Text.Json
open System.Collections
open CodingAdventures.HistoricalWeb.FSharp
open CodingAdventures.TestBootstrap.FSharp
open CodingAdventures.TestBootstrap.FSharp.Fuzzing
open Xunit

module private Helpers =
    let fixture () =
        let rec findRoot dir =
            if File.Exists(Path.Combine(dir, "code/specs/fixtures/historical-web-v1/vectors.json")) then dir
            else
                let parent = Directory.GetParent dir
                if isNull parent then failwith "cannot locate canonical HWEB00 fixture"
                findRoot parent.FullName
        let root = findRoot AppContext.BaseDirectory
        let path = Path.Combine(root, "code/specs/fixtures/historical-web-v1/vectors.json")
        use document = JsonDocument.Parse(File.ReadAllBytes path)
        document.RootElement.Clone()

    let bytes (hex: string) =
        if hex.Length % 2 <> 0 then failwithf "odd-length fixture hex: %s" hex
        Array.init (hex.Length / 2) (fun i -> Convert.ToByte(hex.Substring(i * 2, 2), 16))

    let encodeRequest (data: byte[]) = Http09.encodeRequest(ReadOnlySpan<byte>(data))
    let parseRequest (data: byte[]) = Http09.parseRequest(ReadOnlySpan<byte>(data))
    let decodeResponse (data: byte[]) = Http09.decodeResponse(ReadOnlySpan<byte>(data))

    let httpOracle (wire: byte[]) =
        if wire.Length < 4 || wire[0] <> byte 'G' || wire[1] <> byte 'E' || wire[2] <> byte 'T' || wire[3] <> byte ' ' then None
        else
            let finish =
                if wire.Length >= 6 && wire[wire.Length - 2] = 13uy && wire[wire.Length - 1] = 10uy then wire.Length - 2
                elif wire.Length >= 5 && wire[wire.Length - 1] = 10uy then
                    let beforeLf = wire.Length - 1
                    if beforeLf > 4 && wire[beforeLf - 1] = 13uy then beforeLf - 1 else beforeLf
                else -1
            if finish <= 4 then None
            else
                let address = wire[4 .. finish - 1]
                if address |> Array.forall (fun b -> b >= 0x21uy && b <= 0x7euy) then Some address else None

    let prop (e: JsonElement) (name: string) =
        match e.TryGetProperty name with
        | true, value -> value
        | _ -> failwithf "fixture property missing: %s" name

    let rec checkNode (doc: HtmlDocument) (expected: JsonElement) (index: int) =
        let node = doc.NodeAt index
        if expected.ValueKind = JsonValueKind.String then
            Assert.Equal(HtmlNodeKind.Text, node.Kind)
            Assert.Equal(expected.GetString(), node.Text)
            Assert.Equal(-1, node.FirstChildIndex)
            Assert.Equal(0, node.AttributeCount)
        else
            Assert.Equal(JsonValueKind.Object, expected.ValueKind)
            Assert.Equal(HtmlNodeKind.Element, node.Kind)
            let expectedTag = prop expected "tag" |> fun x -> x.GetString()
            Assert.Equal(expectedTag, node.Tag)
            let attrs = prop expected "attributes"
            let expectedAttrs = attrs.EnumerateObject() |> Seq.map (fun p -> p.Name, (if p.Value.ValueKind = JsonValueKind.Null then None else Some(p.Value.GetString()))) |> Seq.toArray
            Assert.Equal(expectedAttrs.Length, node.AttributeCount)
            let actualAttrs =
                if node.FirstAttributeIndex < 0 then [||]
                else Array.init node.AttributeCount (fun i -> doc.AttributeAt(node.FirstAttributeIndex + i))
            Assert.Equal(expectedAttrs.Length, actualAttrs.Length)
            for i = 0 to expectedAttrs.Length - 1 do
                Assert.Equal(fst expectedAttrs[i], actualAttrs[i].Name)
                Assert.Equal(snd expectedAttrs[i], actualAttrs[i].Value)
            let children = prop expected "children" |> fun x -> x.EnumerateArray() |> Seq.toArray
            let actualChildren = ResizeArray<int>()
            let mutable child = node.FirstChildIndex
            while child >= 0 do
                actualChildren.Add child
                child <- doc.NodeAt(child).NextSiblingIndex
            Assert.Equal(children.Length, actualChildren.Count)
            for i = 0 to children.Length - 1 do checkNode doc children[i] actualChildren[i]

    let checkTree (doc: HtmlDocument) (expected: JsonElement array) =
        Assert.Equal(expected.Length, doc.RootCount)
        let expectedFirst = if expected.Length = 0 then -1 else 0
        Assert.Equal(expectedFirst, doc.FirstRoot)
        let roots = ResizeArray<int>()
        let mutable root = if doc.RootCount = 0 then -1 else doc.FirstRoot
        while root >= 0 do
            roots.Add root
            root <- doc.NodeAt(root).NextSiblingIndex
        Assert.Equal(expected.Length, roots.Count)
        for i = 0 to expected.Length - 1 do checkNode doc expected[i] roots[i]

    let isValidArena (doc: HtmlDocument) =
        let count = doc.NodeCount
        let mutable ok = doc.Nodes.Count = count && doc.Attributes.Count = doc.AttributeCount
        ok <- ok && (if count = 0 then doc.FirstRoot = -1 && doc.RootCount = 0 else doc.FirstRoot = 0)
        for i = 0 to count - 1 do
            let n = doc.NodeAt i
            ok <- ok && (n.ParentIndex = -1 || (n.ParentIndex >= 0 && n.ParentIndex < i))
            ok <- ok && (n.FirstChildIndex = -1 || (n.FirstChildIndex > i && n.FirstChildIndex < count))
            ok <- ok && (n.NextSiblingIndex = -1 || (n.NextSiblingIndex > i && n.NextSiblingIndex < count))
            ok <- ok && (n.FirstAttributeIndex = -1 && n.AttributeCount = 0 || (n.FirstAttributeIndex >= 0 && n.FirstAttributeIndex + n.AttributeCount <= doc.AttributeCount))
            let mutable child = n.FirstChildIndex
            while child >= 0 do
                ok <- ok && child < count && doc.NodeAt(child).ParentIndex = i
                child <- doc.NodeAt(child).NextSiblingIndex
        ok

open Helpers

type SharedVectors() =
    [<Fact>]
    member _.``all shared HTTP vectors replay exactly and fixture failures are fail closed``() =
        let root = fixture ()
        let http = prop root "http-0.9"
        let requests = prop http "requests" |> fun x -> x.EnumerateArray() |> Seq.toArray
        Assert.NotEmpty requests
        for item in requests do
            let address = prop item "address" |> fun x -> x.GetString() |> Encoding.ASCII.GetBytes
            let fixtureWire = bytes (prop item "wire_hex" |> fun x -> x.GetString())
            let canonical = Array.concat [ Encoding.ASCII.GetBytes "GET "; address; [| 13uy; 10uy |] ]
            Assert.Equal<byte>(canonical, encodeRequest address)
            Assert.Equal<byte>(address, parseRequest fixtureWire)
        let invalidAddresses = prop http "invalid_addresses" |> fun x -> x.EnumerateArray() |> Seq.toArray
        Assert.NotEmpty invalidAddresses
        for item in invalidAddresses do
            let address = item.GetString() |> Encoding.UTF8.GetBytes
            let error = Assert.Throws<HistoricalWebException>(fun () -> encodeRequest address |> ignore)
            Assert.Equal(HistoricalWebErrorKind.InvalidAddress, error.Kind)
        let invalidRequests = prop http "invalid_request_hex" |> fun x -> x.EnumerateArray() |> Seq.toArray
        Assert.NotEmpty invalidRequests
        for item in invalidRequests do
            let error = Assert.Throws<HistoricalWebException>(fun () -> parseRequest(bytes (item.GetString())) |> ignore)
            Assert.Equal(HistoricalWebErrorKind.InvalidRequest, error.Kind)
        let responses = prop http "responses" |> fun x -> x.EnumerateArray() |> Seq.toArray
        Assert.NotEmpty responses
        for item in responses do
            let raw = bytes (prop item "wire_hex" |> fun x -> x.GetString())
            let expectedBody = prop item "body" |> fun x -> x.GetString()
            Assert.Equal(expectedBody, decodeResponse raw)
        let invalidResponses = prop http "invalid_response_hex" |> fun x -> x.EnumerateArray() |> Seq.toArray
        Assert.NotEmpty invalidResponses
        for item in invalidResponses do
            let error = Assert.Throws<HistoricalWebException>(fun () -> decodeResponse(bytes (item.GetString())) |> ignore)
            Assert.Equal(HistoricalWebErrorKind.InvalidResponse, error.Kind)

    [<Fact>]
    member _.``all shared HTML source and exact ordered arena trees replay``() =
        let root = fixture ()
        let cases = prop root "html-tags-1992" |> fun x -> x.EnumerateArray() |> Seq.toArray
        Assert.NotEmpty cases
        for item in cases do
            let source = prop item "source" |> fun x -> x.GetString()
            let expected = prop item "tree" |> fun x -> x.EnumerateArray() |> Seq.toArray
            let actual = HtmlTags1992.parse source
            Assert.True(isValidArena actual, "arena links must be in range and parent-consistent")
            checkTree actual expected
        let exchanges = prop root "exchanges" |> fun x -> x.EnumerateArray() |> Seq.toArray
        Assert.NotEmpty exchanges
        for exchange in exchanges do
            let address = prop exchange "address" |> fun x -> Encoding.ASCII.GetBytes(x.GetString())
            let requestWire = bytes (prop exchange "request_hex" |> fun x -> x.GetString())
            Assert.Equal<byte>(address, parseRequest requestWire)
            let response = bytes (prop exchange "response_hex" |> fun x -> x.GetString())
            let body = decodeResponse response
            let expected = prop exchange "tree" |> fun x -> x.EnumerateArray() |> Seq.toArray
            checkTree (HtmlTags1992.parse body) expected

type PublicContracts() =
    [<Fact>]
    member _.``HTTP boundaries report typed errors and preserve octets``() =
        let e = Assert.Throws<HistoricalWebException>(fun () -> encodeRequest [||] |> ignore)
        Assert.Equal(HistoricalWebErrorKind.InvalidAddress, e.Kind)
        let bad = Assert.Throws<HistoricalWebException>(fun () -> parseRequest(Encoding.ASCII.GetBytes "GET /x\r") |> ignore)
        Assert.Equal(HistoricalWebErrorKind.InvalidRequest, bad.Kind)
        let response = Assert.Throws<HistoricalWebException>(fun () -> decodeResponse [| 65uy; 128uy |] |> ignore)
        Assert.Equal(HistoricalWebErrorKind.InvalidResponse, response.Kind)
        Assert.Equal(1, response.Offset.Value)
        Assert.Equal("A\000", decodeResponse [| 65uy; 0uy |])

    [<Fact>]
    member _.``prefix diagnostics and unicode diagnostics identify the first source unit``() =
        let mismatch = Assert.Throws<HistoricalWebException>(fun () -> parseRequest(Encoding.ASCII.GetBytes "GEt /x\n") |> ignore)
        Assert.Equal(HistoricalWebErrorKind.InvalidRequest, mismatch.Kind)
        Assert.Equal(Some 2, mismatch.Offset)
        let short = Assert.Throws<HistoricalWebException>(fun () -> parseRequest(Encoding.ASCII.GetBytes "GE") |> ignore)
        Assert.Equal(Some 2, short.Offset)
        let highAtEnd = Assert.Throws<HistoricalWebException>(fun () -> HtmlTags1992.parse (String [| 'x'; char 0xD800 |]) |> ignore)
        Assert.Equal(Some 1, highAtEnd.Offset)

    [<Fact>]
    member _.``all UTF-16 code units are admitted exactly except isolated surrogates``() =
        for value in 0 .. 65535 do
            let c = char value
            let source = String [| c |]
            if Char.IsSurrogate c then
                let e = Assert.Throws<HistoricalWebException>(fun () -> HtmlTags1992.parse source |> ignore)
                Assert.Equal(HistoricalWebErrorKind.InvalidUnicode, e.Kind)
                Assert.Equal(Some 0, e.Offset)
            else
                let doc = HtmlTags1992.parse source
                Assert.Equal(1, doc.RootCount)
                Assert.Equal(source, doc.NodeAt(doc.FirstRoot).Text)

    [<Fact>]
    member _.``paired scalars NUL, coalescing, valueless and empty attrs are preserved``() =
        let source = "\u0000😀prefix<TITLE>head</TITLE>suffix<A NAME HREF=/index NAME='' TYPE=x HREF=/ignored>left<!-- comment --><unknown>right</A>"
        let doc = HtmlTags1992.parse source
        Assert.True(isValidArena doc)
        Assert.Equal("\u0000😀prefix", doc.NodeAt(doc.FirstRoot).Text)
        let anchor = doc.Nodes |> Seq.find (fun n -> n.Kind = HtmlNodeKind.Element && n.Tag = "a")
        let attrs = Array.init anchor.AttributeCount (fun i -> doc.AttributeAt(anchor.FirstAttributeIndex + i))
        Assert.Equal<string option>(None, attrs[0].Value)
        Assert.Equal(Some "/index", attrs[1].Value)
        Assert.Equal(Some "x", attrs[2].Value)
        Assert.Equal("leftright", doc.NodeAt(anchor.FirstChildIndex).Text)

    [<Fact>]
    member _.``list and glossary omission boundaries choose the newest active scope``() =
        let doc = HtmlTags1992.parse "<UL><LI>outer<MENU><LI>inner</MENU><LI>next</UL><DL><DT>a<DL><DD>b<DT>c</DL><DD>d</DL>"
        Assert.True(isValidArena doc)
        let ul = doc.NodeAt doc.FirstRoot
        Assert.Equal("ul", ul.Tag)
        let entries = doc.Nodes |> Seq.filter (fun n -> n.Kind = HtmlNodeKind.Element && (n.Tag = "li" || n.Tag = "dt" || n.Tag = "dd")) |> Seq.toArray
        Assert.Equal(7, entries.Length)

    [<Fact>]
    member _.``deep container nesting builds an iterative valid arena``() =
        let depth = 12000
        let source = String.replicate depth "<ADDRESS>" + "x</A></A>" + String.replicate depth "</ADDRESS>"
        let doc = HtmlTags1992.parse source
        Assert.Equal(depth + 1, doc.NodeCount)
        Assert.True(isValidArena doc)
        Assert.Equal("x", doc.NodeAt(doc.NodeCount - 1).Text)

    [<Fact>]
    member _.``published arena collections are read-only and bounds checked``() =
        let doc = HtmlTags1992.parse "<A HREF=x>body</A>"
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> doc.NodeAt(-1) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> doc.AttributeAt(doc.AttributeCount) |> ignore) |> ignore
        Assert.False(doc.Nodes :? HtmlNode array)
        Assert.False(doc.Attributes :? HtmlAttribute array)
        Assert.False(doc.Nodes :? System.Collections.Generic.IList<HtmlNode>)
        Assert.False(doc.Attributes :? System.Collections.Generic.IList<HtmlAttribute>)
        Assert.False(doc.Nodes :? ICollection)
        Assert.False(doc.Attributes :? ICollection)

type BoundedExhaustiveAndCampaigns() =
    [<Fact>]
    member _.``all 131586 zero to two octet payloads under LF and CRLF match independent oracle``() =
        let mutable checkedPairs = 0
        for length in 0 .. 2 do
            let total = pown 256 length
            for word in 0 .. total - 1 do
                let payload = Array.init length (fun i -> byte ((word >>> (8 * i)) &&& 255))
                for ending in [| [| 10uy |]; [| 13uy; 10uy |] |] do
                    let wire = Array.concat [ Encoding.ASCII.GetBytes "GET "; payload; ending ]
                    let peeled = if ending.Length = 1 && payload.Length > 0 && payload[payload.Length - 1] = 13uy then payload[0 .. payload.Length - 2] else payload
                    let expected = peeled.Length > 0 && (peeled |> Array.forall (fun b -> b >= 0x21uy && b <= 0x7euy))
                    try
                        let actual = parseRequest wire
                        Assert.True(expected, Convert.ToHexString wire)
                        Assert.Equal<byte>(peeled, actual)
                    with :? HistoricalWebException as e ->
                        Assert.False(expected, sprintf "unexpected rejection %s" (Convert.ToHexString wire))
                        Assert.Equal(HistoricalWebErrorKind.InvalidRequest, e.Kind)
                    checkedPairs <- checkedPairs + 1
        Assert.Equal(131586, checkedPairs)

    [<Fact>]
    member _.``VER05 seeded property checks HTTP and HTML invariants``() =
        let generator = Gen.bytes 0 48
        let verdict = Prop.check 0x48454211UL { Options.defaults with NumTests = 500 } generator (fun bytes ->
            let httpOkay =
                try
                    let address = parseRequest bytes
                    match httpOracle bytes with
                    | Some expected -> expected = address && parseRequest(encodeRequest address) = address
                    | None -> false
                with :? HistoricalWebException -> httpOracle bytes |> Option.isNone
            let doc = HtmlTags1992.parse(Encoding.Latin1.GetString bytes)
            httpOkay && isValidArena doc)
        Assert.True(Verdict.isPassed verdict, sprintf "%A" verdict)

    [<Fact>]
    member _.``VER05 generates guaranteed-valid printable addresses``() =
        let addresses = Gen.map (fun bytes -> bytes |> Array.map (fun b -> byte (0x21 + int b % 94))) (Gen.bytes 1 64)
        let verdict = Prop.check 0x48454213UL { Options.defaults with NumTests = 400 } addresses (fun address ->
            parseRequest(encodeRequest address) = address)
        Assert.True(Verdict.isPassed verdict, sprintf "%A" verdict)

    [<Fact>]
    member _.``VER06 deterministic semantic fuzz finds no unexpected exceptions or arena damage``() =
        let seeds = [| "GET /x\r\n"; "<UL><LI>a<LI>b</UL>"; "<A HREF=x>t</A>"; "<XMP>é</XMP>"; "<" |] |> Array.map Encoding.UTF8.GetBytes |> Array.toList
        let options = { FuzzOptions.defaults with MaxIterations = 2500; Seed = 0x48454212UL; MaxInputLen = 512 }
        let result = Campaign.run seeds options (fun bytes handle ->
            let oracle = httpOracle bytes
            try
                let parsed = parseRequest bytes
                handle.Edge "http:accept"
                if oracle <> Some parsed then failwith "HTTP oracle disagreement"
            with :? HistoricalWebException as error ->
                handle.Edge "http:reject"
                if oracle.IsSome || error.Kind <> HistoricalWebErrorKind.InvalidRequest then failwith "HTTP rejection disagrees with oracle"
            let text = Encoding.Latin1.GetString bytes
            let doc = HtmlTags1992.parse text
            if doc.RootCount = 0 then handle.Edge "html:empty"
            else handle.Edge "html:root"
            if doc.AttributeCount > 0 then handle.Edge "html:attributes"
            if doc.NodeCount > doc.RootCount then handle.Edge "html:children"
            if not (isValidArena doc) then failwith "invalid arena")
        Assert.True(result.IsClean, result.ToString())
        Assert.Equal(2500, result.Iterations)
        Assert.Equal(CallerReported, result.CoverageTier)
