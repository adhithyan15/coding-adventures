namespace UnixHistorySimulator;

/// <summary>A slot owns its offset, so repeated opens never share read position.</summary>
public sealed record Descriptor(int Inode, int Offset);
public sealed class DescriptorTable
{
    private readonly Descriptor?[] _slots;
    public DescriptorTable(int capacity) { if (capacity < 0) throw new ArgumentOutOfRangeException(nameof(capacity)); _slots = new Descriptor?[capacity]; }
    public Descriptor? Get(int fd) => fd >= 0 && fd < _slots.Length ? _slots[fd] : null;
    public int? Open(int inode) { for (var i = 0; i < _slots.Length; i++) if (_slots[i] is null) { _slots[i] = new(inode, 0); return i; } return null; }
    public bool Advance(int fd, int count) { var slot = Get(fd); if (slot is null || count < 0 || count > int.MaxValue - slot.Offset) return false; _slots[fd] = slot with { Offset = slot.Offset + count }; return true; }
    public bool Close(int fd) { if (Get(fd) is null) return false; _slots[fd] = null; return true; }
}

public sealed class Kernel(FileSystem files, DescriptorTable descriptors)
{
    public Dictionary<string, object?> Open(string path)
    {
        var e = new Dictionary<string, object?> { ["call"] = "open", ["path"] = path };
        var (node, error) = files.Resolve(path);
        if (error is not null) e["error"] = error;
        else if (node!.IsDirectory) e["error"] = "EISDIR";
        else if (descriptors.Open(node.Number) is int fd) { e["fd"] = fd; e["inode"] = node.Number; e["offset"] = 0; }
        else e["error"] = "EMFILE";
        return e;
    }
    public (Dictionary<string, object?> Event, string Data) Read(int fd, int count)
    {
        var e = new Dictionary<string, object?> { ["call"] = "read", ["fd"] = fd };
        var slot = descriptors.Get(fd); if (slot is null) { e["error"] = "EBADF"; return (e, ""); }
        var (data, error) = files.Read(slot.Inode, slot.Offset, count);
        if (error is not null) { e["error"] = error; return (e, ""); }
        descriptors.Advance(fd, System.Text.Encoding.ASCII.GetByteCount(data));
        e["inode"] = slot.Inode; e["offset"] = slot.Offset; e["bytes"] = data.Length; e["nextOffset"] = slot.Offset + data.Length;
        return (e, data);
    }
    public Dictionary<string, object?> Close(int fd) { var e = new Dictionary<string, object?> { ["call"] = "close", ["fd"] = fd }; if (!descriptors.Close(fd)) e["error"] = "EBADF"; return e; }
    public (IReadOnlyList<string> Names, Dictionary<string, object?> Event) List(string path)
    {
        var e = new Dictionary<string, object?> { ["call"] = "list", ["path"] = path }; var (node, error) = files.Resolve(path);
        if (error is not null) { e["error"] = error; return (Array.Empty<string>(), e); }
        if (!node!.IsDirectory) { e["error"] = "ENOTDIR"; return (Array.Empty<string>(), e); }
        var names = (node.Entries ?? Array.Empty<Entry>()).Select(x => x.Name).ToArray(); e["names"] = names; e["inode"] = node.Number; return (names, e);
    }
}
