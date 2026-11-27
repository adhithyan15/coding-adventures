namespace UnixHistorySimulator;

public sealed record Result(string Output, IReadOnlyList<Dictionary<string, object?>> Events);

/// <summary>The shell translates teaching commands into kernel operations; it does no host I/O.</summary>
public static class Shell
{
    public static Result RunCommand(string command)
    {
        var kernel = new Kernel(FileSystem.DemoImage(), new DescriptorTable(4));
        var events = new List<Dictionary<string, object?>>(); var output = new System.Text.StringBuilder();
        void Add(Dictionary<string, object?> e) { events.Add(e); if (e.TryGetValue("error", out var error)) output.Append($"error: {error}\n"); }
        var words = command.Split(new[] { ' ', '\t', '\n', '\r', '\f', '\v' }, StringSplitOptions.RemoveEmptyEntries);
        if (words.Length == 0) { }
        else if (words[0] == "cat" && words.Length == 2)
        {
            var opened = kernel.Open(words[1]); Add(opened);
            if (opened.TryGetValue("fd", out var value) && value is int fd)
            {
                while (true) { var (e, data) = kernel.Read(fd, 8); Add(e); if (data.Length == 0) break; output.Append(data); }
                Add(kernel.Close(fd));
            }
        }
        else if (words[0] == "ls" && words.Length <= 2)
        {
            var (names, e) = kernel.List(words.Length == 2 ? words[1] : "/"); Add(e); foreach (var name in names) output.Append(name).Append('\n');
        }
        else output.Append("usage: cat PATH or ls [PATH]\n");
        return new(output.ToString(), events);
    }
}
