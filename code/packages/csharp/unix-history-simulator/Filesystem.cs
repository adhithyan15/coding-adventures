using System.Text;

namespace UnixHistorySimulator;

/// <summary>A directory entry names an inode; entry order is the image's listing order.</summary>
public sealed record Entry(string Name, int Inode);
public sealed record Inode(int Number, bool IsDirectory = false, IReadOnlyList<Entry>? Entries = null, string Data = "");

/// <summary>Immutable-in-shape teaching image. Inode data represents bytes in this ASCII demo.</summary>
public sealed class FileSystem
{
    private readonly IReadOnlyDictionary<int, Inode> _inodes;
    public FileSystem(IEnumerable<Inode> inodes) => _inodes = inodes
        .Select(node => node with
        {
            Entries = node.Entries is null ? null : Array.AsReadOnly(node.Entries.ToArray())
        })
        .ToDictionary(n => n.Number);
    public static FileSystem DemoImage() => new(new[] {
        new Inode(1, true, new[] { new Entry("hello", 2), new Entry("empty", 3), new Entry("etc", 4) }),
        new Inode(2, Data: "Hello, Unix!\n"), new Inode(3),
        new Inode(4, true, new[] { new Entry("notes", 5) }),
        new Inode(5, Data: "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n")
    });

    public (Inode? Node, string? Error) Resolve(string path)
    {
        if (string.IsNullOrEmpty(path) || path.Contains('\0')) return (null, "EINVAL");
        if (!_inodes.TryGetValue(1, out var current)) return (null, "ENOENT");
        var parents = new Stack<Inode>();
        foreach (var name in path.Split('/').Where(x => x.Length > 0))
        {
            if (!current.IsDirectory) return (null, "ENOTDIR");
            if (name == ".") continue;
            if (name == "..") { if (parents.Count > 0) current = parents.Pop(); continue; }
            var entry = (current.Entries ?? Array.Empty<Entry>()).FirstOrDefault(e => e.Name == name);
            if (entry is null || !_inodes.TryGetValue(entry.Inode, out var next)) return (null, "ENOENT");
            parents.Push(current); current = next;
        }
        if (path.EndsWith('/') && !current.IsDirectory) return (null, "ENOTDIR");
        return (current, null);
    }

    public (string Data, string? Error) Read(int inode, int offset, int count)
    {
        if (offset < 0 || count < 0) return ("", "EINVAL");
        if (!_inodes.TryGetValue(inode, out var file)) return ("", "ENOENT");
        if (file.IsDirectory) return ("", "EISDIR");
        var bytes = Encoding.ASCII.GetBytes(file.Data);
        if (offset >= bytes.Length) return ("", null);
        return (Encoding.ASCII.GetString(bytes, offset, Math.Min(count, bytes.Length - offset)), null);
    }
}
