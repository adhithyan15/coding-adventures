using System.Text.Json;
using UnixHistorySimulator;

public static class FixtureCli
{
    public static int Main(string[] args)
    {
        if (args.Length == 1 && args[0] == "--self-test")
            return SelfTest() ? 0 : 1;
        if (args.Length != 1)
        {
            Console.Error.WriteLine("usage: FixtureCli COMMAND");
            return 2;
        }

        var result = Shell.RunCommand(args[0]);
        Console.WriteLine(JsonSerializer.Serialize(result, new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase
        }));
        return 0;
    }

    private static bool SelfTest()
    {
        try
        {
            _ = new DescriptorTable(-1);
            return false;
        }
        catch (ArgumentOutOfRangeException)
        {
        }

        var fs = FileSystem.DemoImage();
        var table = new DescriptorTable(2);
        var kernel = new Kernel(fs, table);

        var sourceEntries = new List<Entry> { new("original", 2) };
        var copiedFs = new FileSystem(new[]
        {
            new Inode(1, true, sourceEntries),
            new Inode(2, Data: "x")
        });
        sourceEntries[0] = new Entry("mutated-input", 2);
        var exposedEntries = copiedFs.Resolve("/").Node?.Entries;
        if (copiedFs.Resolve("/original").Node is null || exposedEntries is null)
            return false;
        try
        {
            ((IList<Entry>)exposedEntries)[0] = new Entry("mutated-output", 2);
            return false;
        }
        catch (NotSupportedException)
        {
        }
        if (copiedFs.Resolve("/original").Node is null)
            return false;

        if (fs.Resolve("").Error != "EINVAL" || fs.Resolve("/bad\0path").Error != "EINVAL" ||
            fs.Resolve("/hello/..").Error != "ENOTDIR" || fs.Resolve("/empty").Node?.Number != 3 ||
            fs.Read(1, 0, 1).Error != "EISDIR" || fs.Read(2, -1, 1).Error != "EINVAL")
            return false;

        var first = table.Open(2)!.Value;
        var second = table.Open(2)!.Value;
        if (first != 0 || second != 1 || table.Open(2) is not null || table.Get(-1) is not null ||
            !table.Advance(first, 3) || table.Advance(first, -1) || table.Get(first)?.Offset != 3 ||
            table.Get(second)?.Offset != 0 || !table.Close(first) || table.Close(first) ||
            table.Open(2) != 0 || kernel.Read(99, 4).Event.GetValueOrDefault("error") as string != "EBADF")
            return false;

        return true;
    }
}
