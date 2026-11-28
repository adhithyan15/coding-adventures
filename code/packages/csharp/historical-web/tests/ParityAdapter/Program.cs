using System.Text;
using System.Text.Json;
using CodingAdventures.HistoricalWeb;

namespace CodingAdventures.HistoricalWeb.ParityAdapter;

internal static class Program
{
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);

    private static object? ToTreeNode(HtmlDocument document, int index)
    {
        HtmlNode node = document.NodeAt(index);
        if (node.Kind == HtmlNodeKind.Text)
        {
            return node.Text;
        }

        Dictionary<string, string?> attributes = new(StringComparer.Ordinal);
        for (int i = 0; i < node.AttributeCount; i++)
        {
            HtmlAttribute attribute = document.AttributeAt(node.FirstAttributeIndex + i);
            attributes.Add(attribute.Name, attribute.Value);
        }

        List<object?> children = new();
        int child = node.FirstChildIndex;
        while (child != -1)
        {
            children.Add(ToTreeNode(document, child));
            child = document.NodeAt(child).NextSiblingIndex;
        }

        return new { tag = node.Tag, attributes, children };
    }

    private static object ToTree(HtmlDocument document)
    {
        List<object?> roots = new();
        int root = document.FirstRoot;
        while (root != -1)
        {
            roots.Add(ToTreeNode(document, root));
            root = document.NodeAt(root).NextSiblingIndex;
        }

        return roots;
    }

    public static int Main()
    {
        string? line;
        while ((line = Console.ReadLine()) != null)
        {
            try
            {
                string source = StrictUtf8.GetString(Convert.FromHexString(line));
                HtmlDocument document = HtmlTags1992.Parse(source);
                Console.WriteLine(JsonSerializer.Serialize(ToTree(document)));
            }
            catch
            {
                Console.WriteLine("null");
            }
        }

        return 0;
    }
}
