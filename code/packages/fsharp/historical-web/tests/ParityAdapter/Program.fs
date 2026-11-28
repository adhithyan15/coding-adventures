open System
open System.Text.Json
open CodingAdventures.HistoricalWeb.FSharp

let options = JsonSerializerOptions(PropertyNamingPolicy = JsonNamingPolicy.CamelCase)

let tree (doc: HtmlDocument) =
    let rec node index =
        let item = doc.NodeAt index
        if item.Kind = HtmlNodeKind.Text then box item.Text
        else
            let attrs = System.Collections.Generic.Dictionary<string, string option>()
            if item.FirstAttributeIndex >= 0 then
                for offset in 0 .. item.AttributeCount - 1 do
                    let attr = doc.AttributeAt(item.FirstAttributeIndex + offset)
                    attrs.Add(attr.Name, attr.Value)
            let children = ResizeArray<obj>()
            let mutable child = item.FirstChildIndex
            while child >= 0 do
                children.Add(node child)
                child <- doc.NodeAt(child).NextSiblingIndex
            box {| tag = item.Tag; attributes = attrs; children = children.ToArray() |}
    let roots = ResizeArray<obj>()
    let mutable root = if doc.RootCount = 0 then -1 else doc.FirstRoot
    while root >= 0 do
        roots.Add(node root)
        root <- doc.NodeAt(root).NextSiblingIndex
    roots.ToArray()

while true do
    let line = Console.ReadLine()
    if isNull line then Environment.Exit 0
    use parsed = JsonDocument.Parse line
    let source = parsed.RootElement.GetString()
    let result = HtmlTags1992.parse source |> tree
    Console.WriteLine(JsonSerializer.Serialize(result, options))
