// Behaviour of the XAML platform library (UI87 §7.6) without a display: the
// pickers are replaced by a fake that returns a chosen path (or cancels), and
// the host by a fake that records deferrals and answers, so the real
// open/save logic, limits and routing run as they would in an app -- the same
// cases as the Compose library's MosaicPlatformEffectsTest.kt and the SwiftUI
// harness's PlatformEffectsChecks.swift.
//
// Compiled with MOSAIC_HEADLESS_TEST (see the .csproj), after copying a
// generated WinUI project's MosaicPlatformEffects.cs and MosaicRuntimeHost.cs
// beside this file:
//
//   dotnet run --project XamlPlatformEffectsConformance.csproj
//
// CI does this in the Windows XAML lane, so the save's move-into-place runs on
// NTFS, the file system the real app writes to. The POSIX permission checks
// run wherever the harness runs on Unix.
using System.Text;
using System.Text.Json;

namespace Mosaic.Generated;

internal sealed class FakeDialogs : IMosaicFileDialogs
{
    private readonly string? choice;
    private readonly bool throws;

    public FakeDialogs(string? choice, bool throws = false)
    {
        this.choice = choice;
        this.throws = throws;
    }

    public int Opened { get; private set; }
    public IReadOnlyList<string> LastExtensions { get; private set; } = Array.Empty<string>();
    public string? LastSuggestedName { get; private set; }

    public Task<string?> ChooseFileToOpenAsync(IReadOnlyList<string> extensions)
    {
        Opened += 1;
        LastExtensions = extensions.ToList();
        if (throws) throw new InvalidOperationException("C:\\Users\\someone\\secret path");
        return Task.FromResult(choice);
    }

    public Task<string?> ChooseFileToSaveAsync(string suggestedName, IReadOnlyList<string> extensions)
    {
        Opened += 1;
        LastExtensions = extensions.ToList();
        LastSuggestedName = suggestedName;
        if (throws) throw new InvalidOperationException("C:\\Users\\someone\\secret path");
        return Task.FromResult(choice);
    }
}

internal sealed class FakeHost : IMosaicPlatformEffectHost
{
    public Action<ulong, string, JsonElement, string>? EffectHandler { get; set; }
    public HashSet<ulong> WaitingOn { get; set; } = new();
    public List<ulong> Deferred { get; } = new();
    public Dictionary<ulong, string> Answers { get; } = new();
    public bool ThrowOnComplete { get; set; }

    public bool DeferEffect(ulong id)
    {
        if (!WaitingOn.Contains(id)) return false;
        Deferred.Add(id);
        return true;
    }

    public void CompleteEffect(ulong id, object result)
    {
        if (ThrowOnComplete) throw new InvalidOperationException("the runtime is closed");
        // Recorded as the JSON the real host would hand the runtime.
        Answers[id] = JsonSerializer.Serialize(result);
        WaitingOn.Remove(id);
    }
}

internal static class Program
{
    private static int checks;

    private static void Check(bool condition, string assertion)
    {
        checks += 1;
        if (!condition) throw new InvalidOperationException($"Failed platform-effects assertion: {assertion}");
    }

    private static string Encoded(string text) => Convert.ToBase64String(Encoding.UTF8.GetBytes(text));

    private static JsonElement Payload(Dictionary<string, object?> fields) =>
        JsonSerializer.SerializeToElement(fields);

    private static readonly JsonElement NoPayload = default;

    private static Dictionary<string, object?> Open(JsonElement payload, IMosaicFileDialogs dialogs) =>
        MosaicPlatformEffects.RunFilesOpenAsync(payload, dialogs).GetAwaiter().GetResult();

    private static Dictionary<string, object?> Save(JsonElement payload, IMosaicFileDialogs dialogs) =>
        MosaicPlatformEffects.RunFilesSaveAsync(payload, dialogs).GetAwaiter().GetResult();

    private static string? Failure(Dictionary<string, object?> outcome) =>
        outcome.TryGetValue("failed", out var failed) && failed is Dictionary<string, object?> body
            ? body["message"] as string
            : null;

    private static bool IsCancelled(Dictionary<string, object?> outcome) =>
        outcome.Count == 1
        && outcome.TryGetValue("cancelled", out var cancelled)
        && cancelled is Dictionary<string, object?> { Count: 0 };

    private static Dictionary<string, object?>? OkValue(Dictionary<string, object?> outcome) =>
        outcome.TryGetValue("ok", out var ok) ? ok as Dictionary<string, object?> : null;

    private static string? AnsweredFailure(FakeHost host, ulong id) =>
        host.Answers.TryGetValue(id, out var json)
        && JsonDocument.Parse(json).RootElement is { ValueKind: JsonValueKind.Object } root
        && root.TryGetProperty("failed", out var failed)
            ? failed.GetProperty("message").GetString()
            : null;

    private static string? AnsweredName(FakeHost host, ulong id) =>
        host.Answers.TryGetValue(id, out var json)
        && JsonDocument.Parse(json).RootElement is { ValueKind: JsonValueKind.Object } root
        && root.TryGetProperty("ok", out var ok)
            ? ok.GetProperty("name").GetString()
            : null;

    private static void CheckRouting()
    {
        IReadOnlySet<string> Kinds(params string[] kinds) => new HashSet<string>(kinds);
        // The app claims a kind: it goes to the app, even a standard one.
        Check(MosaicPlatformEffects.RoutesToPlatform("files.save", Kinds("files.save")) == false, "claimed standard");
        // A standard kind nobody claimed: the platform library.
        Check(MosaicPlatformEffects.RoutesToPlatform("files.open", Kinds("importAnki")) == true, "unclaimed open");
        Check(MosaicPlatformEffects.RoutesToPlatform("files.save", null) == true, "unclaimed save");
        // A non-standard kind: the app when it claimed nothing (the original
        // meaning), nobody when it named its kinds and this is not one.
        Check(MosaicPlatformEffects.RoutesToPlatform("importAnki", null) == false, "unclaimed custom");
        Check(MosaicPlatformEffects.RoutesToPlatform("somethingElse", Kinds("importAnki")) is null, "unowned custom");
    }

    private static void CheckSave(string directory)
    {
        var target = Path.Combine(directory, "journal-2026-09-25.json");
        var dialogs = new FakeDialogs(target);
        var outcome = Save(Payload(new()
        {
            ["suggestedName"] = "journal-2026-09-25.json",
            ["accept"] = new[] { "application/json" },
            ["bytes"] = Encoded("{\"version\":1}"),
        }), dialogs);
        Check(OkValue(outcome)?["name"] as string == "journal-2026-09-25.json", "save ok name");
        Check(OkValue(outcome)!.Count == 1, "save answers only the name");
        Check(File.ReadAllText(target) == "{\"version\":1}", "saved bytes");
        Check(dialogs.LastExtensions.SequenceEqual(new[] { "json" }), "save picker filtered to json");
        Check(dialogs.LastSuggestedName == "journal-2026-09-25.json", "suggested name reached picker");
        Check(!Directory.EnumerateFiles(directory).Any(file => file.EndsWith(".tmp", StringComparison.Ordinal)),
            "no temporary file left behind");

        // Saving over an existing file replaces it, in place, whole.
        var existing = Path.Combine(directory, "existing.txt");
        File.WriteAllText(existing, "a much longer old body that must not survive");
        Check(OkValue(Save(Payload(new() { ["suggestedName"] = "existing.txt", ["bytes"] = Encoded("new") }),
            new FakeDialogs(existing))) is not null, "save over an existing file");
        Check(File.ReadAllText(existing) == "new", "replaced contents, nothing of the old left");

        // A target that cannot be written is a failure without the path.
        var folder = Path.Combine(directory, "a-folder.txt");
        Directory.CreateDirectory(folder);
        var blocked = Save(Payload(new() { ["suggestedName"] = "a-folder.txt", ["bytes"] = Encoded("x") }),
            new FakeDialogs(folder));
        Check(Failure(blocked) == "couldn't save the file", $"unwritable target: {Failure(blocked)}");
        Check(!Directory.EnumerateFiles(directory).Any(file => file.EndsWith(".tmp", StringComparison.Ordinal)),
            "a failed save leaves no temporary file");

        if (OperatingSystem.IsWindows()) return;
        // POSIX only: saving over a private file keeps it private.
        var secret = Path.Combine(directory, "secret.json");
        File.WriteAllText(secret, "old");
        File.SetUnixFileMode(secret, UnixFileMode.UserRead | UnixFileMode.UserWrite);
        Check(OkValue(Save(Payload(new() { ["suggestedName"] = "secret.json", ["bytes"] = Encoded("new") }),
            new FakeDialogs(secret))) is not null, "save over a private file");
        Check(File.GetUnixFileMode(secret) == (UnixFileMode.UserRead | UnixFileMode.UserWrite),
            $"replaced file keeps 0600, got {File.GetUnixFileMode(secret)}");
        Check(File.ReadAllText(secret) == "new", "replaced private contents");

        // Only the rwx bits carry over: never setuid, setgid or sticky.
        var tool = Path.Combine(directory, "tool.txt");
        File.WriteAllText(tool, "old");
        const UnixFileMode rwxrxrx = (UnixFileMode)0x1ED; // 0755
        File.SetUnixFileMode(tool, rwxrxrx | UnixFileMode.SetUser);
        Check(OkValue(Save(Payload(new() { ["suggestedName"] = "tool.txt", ["bytes"] = Encoded("new") }),
            new FakeDialogs(tool))) is not null, "save over a setuid file");
        Check(File.GetUnixFileMode(tool) == rwxrxrx, $"setuid is not copied, got {File.GetUnixFileMode(tool)}");

        // A new file stays owner-only.
        var fresh = Path.Combine(directory, "fresh.txt");
        Save(Payload(new() { ["suggestedName"] = "fresh.txt", ["bytes"] = Encoded("x") }), new FakeDialogs(fresh));
        Check(File.GetUnixFileMode(fresh) == (UnixFileMode.UserRead | UnixFileMode.UserWrite),
            $"a new file is 0600, got {File.GetUnixFileMode(fresh)}");
    }

    private static void CheckSaveRefusals(string directory)
    {
        var cancelled = Save(Payload(new() { ["suggestedName"] = "a.json", ["bytes"] = Encoded("x") }),
            new FakeDialogs(null));
        Check(IsCancelled(cancelled), "a cancelled picker is not a failure");

        var target = Path.Combine(directory, "never.txt");
        foreach (var name in new[]
        {
            "", ".", "..", "../escape.json", "dir/a.json", "a\\b.json", "a\u0000b", "bell\u0007.json",
            "D:report.json", "notes.json:stream", "invoice\u202Efdp.exe", "trailing.", "trailing ",
            // Tightened after the SwiftUI library's security review (UI87 §7):
            ".zshrc", " leading.json", "nbsp\u00A0", "\u3000ideographic.json",
            "line\u2028break.json", "para\u2029break.json",
            "tag\uDB40\uDC01.json", // U+E0001 LANGUAGE TAG, a format character outside the BMP
            new string('x', 256), "Invoice.pdf      .command",
            ".\u0301zshrc", "Invoice.pdf\u2800\u2800.txt", "x\u0D4E.",
            "a\uD800.json", "a\uDC00.json", "a\uE000.json",
            "Invoice.pdf" + string.Concat(Enumerable.Repeat(" \uFE00", 30)) + " x.html", "\uFE00.zshrc",
            "notes.txt\uFE00.",
        })
        {
            Check(!MosaicPlatformEffects.IsPlainFileName(name), $"plain name refused: {Escape(name)}");
            // Written as JSON by hand, every non-ASCII unit as `\uXXXX`: the
            // serializer would quietly turn a lone surrogate into U+FFFD (a
            // name that passes), and the point is what arrives on the wire.
            var payload = JsonDocument.Parse(
                $"{{\"suggestedName\":\"{JsonEscape(name)}\",\"bytes\":\"{Encoded("x")}\"}}").RootElement;
            var dialogs = new FakeDialogs(target);
            var outcome = Save(payload, dialogs);
            Check(Failure(outcome) == "suggestedName must be a plain file name", $"refused name {Escape(name)}");
            Check(dialogs.Opened == 0, $"no picker for refused name {Escape(name)}");
        }
        // No suggestedName at all, or not a string, is refused the same way.
        Check(Failure(Save(Payload(new() { ["bytes"] = Encoded("x") }), new FakeDialogs(target)))
            == "suggestedName must be a plain file name", "missing suggestedName");
        Check(Failure(Save(Payload(new() { ["suggestedName"] = 7, ["bytes"] = Encoded("x") }), new FakeDialogs(target)))
            == "suggestedName must be a plain file name", "non-string suggestedName");

        var mismatched = Save(Payload(new()
        {
            ["suggestedName"] = "notes.exe", ["accept"] = new[] { "application/json" }, ["bytes"] = Encoded("{}"),
        }), new FakeDialogs(Path.Combine(directory, "notes.exe")));
        Check(Failure(mismatched) == "suggestedName must end in an extension of an accepted type",
            "extension must match the accepted type");
        Check(!File.Exists(Path.Combine(directory, "notes.exe")), "a mismatched name writes nothing");

        // With no accepted type, a name that would run when opened is refused.
        foreach (var name in new[]
        {
            "run.command", "open.terminal", "site.webloc", "setup.EXE", "go.desktop", "a.ps1",
            "a.j\u017F", "img.iso", "clip.scf", "app.AppImage", "run\u0D4E.terminal",
        })
        {
            Check(MosaicPlatformEffects.HasExecutableExtension(name), $"executable: {Escape(name)}");
            var dialogs = new FakeDialogs(Path.Combine(directory, name));
            var outcome = Save(Payload(new() { ["suggestedName"] = name, ["bytes"] = Encoded("x") }), dialogs);
            Check(Failure(outcome) == "suggestedName must not end in an executable extension",
                $"executable extension {Escape(name)}: {Failure(outcome)}");
            Check(dialogs.Opened == 0, $"no picker for executable {Escape(name)}");
            Check(!File.Exists(Path.Combine(directory, name)), $"nothing written for {Escape(name)}");
        }
        Check(MosaicPlatformEffects.IsPlainFileName("journal.json"), "an ordinary name passes");
        Check(MosaicPlatformEffects.IsPlainFileName("caf\u00E9 menu.json"), "accented names still pass");
        Check(MosaicPlatformEffects.IsPlainFileName("\u2764\uFE0F list.txt"), "an emoji's own selector is fine");
        Check(MosaicPlatformEffects.IsPlainFileName(new string('x', 255)), "255 UTF-16 units is the limit");
        Check(!MosaicPlatformEffects.HasExecutableExtension("notes.txt")
            && !MosaicPlatformEffects.HasExecutableExtension("README"), "an ordinary document is not executable");

        foreach (var bad in new[] { "not base64!", "%%%", "YQ", "YQ==\n", "YW Jj" })
        {
            var outcome = Save(Payload(new() { ["suggestedName"] = "a.txt", ["bytes"] = bad }), new FakeDialogs(target));
            Check(Failure(outcome) == "bytes must be base64 text", $"invalid base64 {Escape(bad)}");
        }
        Check(Failure(Save(Payload(new() { ["suggestedName"] = "a.txt" }), new FakeDialogs(target)))
            == "bytes must be base64 text", "missing bytes");

        var tooLarge = new string('A', (MosaicPlatformEffects.MaxSaveBytes / 3 + 2) * 4);
        var oversized = Save(Payload(new() { ["suggestedName"] = "a.txt", ["bytes"] = tooLarge }), new FakeDialogs(target));
        Check(Failure(oversized) == "the file is larger than 16777216 bytes", $"oversized save: {Failure(oversized)}");
        Check(!File.Exists(target), "refusals write nothing");
    }

    private static void CheckOpen(string directory)
    {
        var source = Path.Combine(directory, "photo.PNG");
        File.WriteAllBytes(source, new byte[] { 1, 2, 3 });
        var dialogs = new FakeDialogs(source);
        var outcome = Open(Payload(new() { ["accept"] = new[] { "image/png", "image/jpeg", "image/png", "x/unknown" } }), dialogs);
        var value = OkValue(outcome);
        Check(value?["name"] as string == "photo.PNG", "open returns the name, never a path");
        Check(value?["mimeType"] as string == "image/png", "mime type from the extension");
        Check(value?["bytes"] as string == Convert.ToBase64String(new byte[] { 1, 2, 3 }), "open bytes");
        Check(dialogs.LastExtensions.SequenceEqual(new[] { "png", "jpg", "jpeg" }),
            $"extensions deduplicated in order, unknown dropped: {string.Join(",", dialogs.LastExtensions)}");

        // An unknown extension is application/octet-stream; no accept is any file.
        var unknown = Path.Combine(directory, "notes.xyz");
        File.WriteAllText(unknown, "x");
        var anyFile = new FakeDialogs(unknown);
        Check(OkValue(Open(NoPayload, anyFile))?["mimeType"] as string == "application/octet-stream",
            "unknown extension");
        Check(anyFile.LastExtensions.Count == 0, "no accept filters nothing");

        Check(IsCancelled(Open(NoPayload, new FakeDialogs(null))), "open cancelled");
        Check(Failure(Open(NoPayload, new FakeDialogs(directory))) == "that is not a regular file",
            "a directory is refused");
        Check(Failure(Open(NoPayload, new FakeDialogs(Path.Combine(directory, "missing.png"))))
            == "that is not a regular file", "a missing file is refused");

        if (!OperatingSystem.IsWindows())
        {
            // A symlink the person chose is read, and named as chosen.
            var link = Path.Combine(directory, "link.png");
            File.CreateSymbolicLink(link, source);
            var linked = OkValue(Open(NoPayload, new FakeDialogs(link)));
            Check(linked?["bytes"] as string == Convert.ToBase64String(new byte[] { 1, 2, 3 }), "chosen symlink");
            Check(linked?["name"] as string == "link.png", "the chosen name, not the link target");
        }

        // Exactly the limit is read; one byte over is refused, bounded while reading.
        var atLimit = Path.Combine(directory, "at-limit.bin");
        using (var stream = File.Create(atLimit)) stream.SetLength(MosaicPlatformEffects.MaxOpenBytes);
        Check(OkValue(Open(NoPayload, new FakeDialogs(atLimit))) is not null, "a file of exactly the limit opens");
        var large = Path.Combine(directory, "large.bin");
        using (var stream = File.Create(large)) stream.SetLength(MosaicPlatformEffects.MaxOpenBytes + 1);
        Check(Failure(Open(NoPayload, new FakeDialogs(large))) == "the selected file is larger than 52428800 bytes",
            "oversized open");
    }

    private static void CheckRouter(string directory)
    {
        var target = Path.Combine(directory, "routed.txt");
        var payload = Payload(new() { ["suggestedName"] = "routed.txt", ["bytes"] = Encoded("hi") });

        // A standard kind is deferred, then answered from the UI queue.
        var queued = new List<Action>();
        var host = new FakeHost();
        var appCalls = new List<string>();
        host.EffectHandler = (_, kind, _, _) => appCalls.Add(kind);
        MosaicPlatformEffects.Install(host, new[] { "importAnki" }, new FakeDialogs(target), work =>
        {
            queued.Add(work);
            return true;
        });
        var router = host.EffectHandler;
        // Idempotent: a second install does not stack a second router.
        MosaicPlatformEffects.Install(host, null, new FakeDialogs(null), work =>
        {
            work();
            return true;
        });
        Check(ReferenceEquals(host.EffectHandler, router), "a second install changes nothing");

        host.WaitingOn = new HashSet<ulong> { 1, 2 };
        host.EffectHandler!(1, "files.save", payload, "await");
        Check(host.Deferred.SequenceEqual(new ulong[] { 1 }), "a standard kind is deferred before any picker");
        Check(!host.Answers.ContainsKey(1) && queued.Count == 1, "answered later, on the UI queue");
        // One file operation at a time: a second request while the first is open.
        host.EffectHandler!(2, "files.save", payload, "await");
        Check(AnsweredFailure(host, 2) == "another file operation is in progress", "busy");
        var first = queued[0];
        queued.Clear();
        first();
        Check(AnsweredName(host, 1) == "routed.txt", "deferred answer");
        Check(File.ReadAllText(target) == "hi", "the deferred save wrote the file");

        // The payload outlives the host's parsed update: answered from a copy.
        using (var document = JsonDocument.Parse(JsonSerializer.Serialize(new Dictionary<string, object?>
               {
                   ["suggestedName"] = "copied.txt", ["bytes"] = Encoded("copy"),
               })))
        {
            host.WaitingOn = new HashSet<ulong> { 9 };
            host.EffectHandler!(9, "files.save", document.RootElement, "await");
        }
        var copiedTarget = Path.Combine(directory, "copied.txt");
        var afterDispose = queued[0];
        queued.Clear();
        // The fake dialog's choice is `routed.txt`; what matters is that the
        // request was still readable after its document was disposed.
        afterDispose();
        Check(AnsweredName(host, 9) == "routed.txt", "a deferred request survives its document");
        Check(File.ReadAllText(target) == "copy", "and saves the copied bytes");
        Check(!File.Exists(copiedTarget), "the picker's choice, not the suggestion, is written");

        // The app's claimed kind reaches the app; an unclaimed custom kind
        // reaches nobody (the host fails it); a notify needs no answer.
        host.EffectHandler!(3, "importAnki", NoPayload, "await");
        host.EffectHandler!(4, "somethingElse", NoPayload, "await");
        host.WaitingOn = new HashSet<ulong> { 5 };
        host.EffectHandler!(5, "files.open", NoPayload, "notify");
        Check(appCalls.SequenceEqual(new[] { "importAnki" }), $"only the claimed kind reaches the app: {string.Join(",", appCalls)}");
        Check(!host.Answers.ContainsKey(4) && !host.Answers.ContainsKey(5), "nothing answered for 4 and 5");
        Check(queued.Count == 0, "a notify opens no picker");

        // Not waiting on the id: no picker, and the router is free again.
        host.WaitingOn = new HashSet<ulong>();
        host.EffectHandler!(6, "files.open", NoPayload, "await");
        Check(queued.Count == 0 && !host.Answers.ContainsKey(6), "an id nobody awaits opens nothing");
        host.WaitingOn = new HashSet<ulong> { 7 };
        host.EffectHandler!(7, "files.open", NoPayload, "AWAIT");
        Check(queued.Count == 1, "the router is not left busy by a refused deferral (and AWAIT is an await)");
        queued[0]();
        queued.Clear();
        Check(host.Answers.ContainsKey(7), "answered after the refused deferral");

        // A claimed standard kind reaches the app, not the library.
        var claimed = new FakeHost();
        var claimedCalls = new List<string>();
        claimed.EffectHandler = (_, kind, _, _) => claimedCalls.Add(kind);
        MosaicPlatformEffects.Install(claimed, new[] { "files.save" }, new FakeDialogs(target), _ =>
            throw new InvalidOperationException("no picker for a claimed kind"));
        claimed.WaitingOn = new HashSet<ulong> { 1, 2 };
        claimed.EffectHandler!(1, "files.save", payload, "await");
        Check(claimedCalls.SequenceEqual(new[] { "files.save" }) && claimed.Deferred.Count == 0,
            "a claimed standard kind goes to the app");
        // The unclaimed one still reaches the library.
        var claimedQueue = new List<Action>();
        var partly = new FakeHost();
        var partlyCalls = new List<string>();
        partly.EffectHandler = (_, kind, _, _) => partlyCalls.Add(kind);
        MosaicPlatformEffects.Install(partly, new[] { "files.save" }, new FakeDialogs(null), work =>
        {
            claimedQueue.Add(work);
            return true;
        });
        partly.WaitingOn = new HashSet<ulong> { 1 };
        partly.EffectHandler!(1, "files.open", NoPayload, "await");
        Check(partlyCalls.Count == 0 && claimedQueue.Count == 1, "the unclaimed standard kind reaches the library");
        claimedQueue[0]();
        Check(partly.Answers[1] == "{\"cancelled\":{}}", $"a cancelled picker answers cancelled: {partly.Answers[1]}");

        // No app handler and no kinds: a custom kind goes to nobody here.
        var bare = new FakeHost();
        MosaicPlatformEffects.Install(bare, null, new FakeDialogs(null), work => { work(); return true; });
        bare.WaitingOn = new HashSet<ulong> { 1 };
        bare.EffectHandler!(1, "importAnki", NoPayload, "await");
        Check(!bare.Answers.ContainsKey(1) && bare.Deferred.Count == 0, "no handler: left for the host's sweep");

        // A queue that refuses the work (a closing window): failed at once,
        // because a deferred effect is out of the host's sweep.
        var closing = new FakeHost();
        MosaicPlatformEffects.Install(closing, null, new FakeDialogs(target), _ => false);
        closing.WaitingOn = new HashSet<ulong> { 1, 2 };
        closing.EffectHandler!(1, "files.open", NoPayload, "await");
        Check(AnsweredFailure(closing, 1) == "the file dialog failed", "a refused queue fails the effect");
        closing.EffectHandler!(2, "files.open", NoPayload, "await");
        Check(AnsweredFailure(closing, 2) == "the file dialog failed", "and does not leave the router busy");

        // A queue that throws is a refused queue too.
        var throwing = new FakeHost();
        MosaicPlatformEffects.Install(throwing, null, new FakeDialogs(target), _ =>
            throw new InvalidOperationException("no dispatcher"));
        throwing.WaitingOn = new HashSet<ulong> { 1 };
        throwing.EffectHandler!(1, "files.save", payload, "await");
        Check(AnsweredFailure(throwing, 1) == "the file dialog failed", "a throwing queue fails the effect");

        // A picker that throws: failed, without the exception's text (a path).
        var broken = new FakeHost();
        MosaicPlatformEffects.Install(broken, null, new FakeDialogs(null, throws: true), work => { work(); return true; });
        broken.WaitingOn = new HashSet<ulong> { 1, 2 };
        broken.EffectHandler!(1, "files.open", NoPayload, "await");
        Check(AnsweredFailure(broken, 1) == "the file dialog failed", "a throwing picker fails the effect");
        broken.EffectHandler!(2, "files.save", payload, "await");
        Check(AnsweredFailure(broken, 2) == "the file dialog failed", "and does not leave the router busy");

        // A host that refuses the answer (closed underneath): nothing escapes.
        var closed = new FakeHost { ThrowOnComplete = true };
        MosaicPlatformEffects.Install(closed, null, new FakeDialogs(null), work => { work(); return true; });
        closed.WaitingOn = new HashSet<ulong> { 1, 2 };
        closed.EffectHandler!(1, "files.open", NoPayload, "await");
        closed.EffectHandler!(2, "files.open", NoPayload, "await");
        Check(closed.Deferred.SequenceEqual(new ulong[] { 1, 2 }), "a refused answer does not leave the router busy");
    }

    private static string JsonEscape(string text)
    {
        var builder = new StringBuilder();
        foreach (var character in text)
        {
            if (character is >= ' ' and <= '~' and not '"' and not '\\') builder.Append(character);
            else builder.Append($"\\u{(int)character:x4}");
        }
        return builder.ToString();
    }

    private static string Escape(string text)
    {
        var builder = new StringBuilder();
        foreach (var character in text.Length > 40 ? text[..40] + "..." : text)
        {
            if (character is >= ' ' and <= '~') builder.Append(character);
            else builder.Append($"\\u{(int)character:X4}");
        }
        return builder.ToString();
    }

    private static void Main()
    {
        var directory = Path.Combine(Path.GetTempPath(), "mosaic-platform-effects-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(directory);
        try
        {
            CheckRouting();
            CheckSave(directory);
            CheckSaveRefusals(directory);
            CheckOpen(directory);
            CheckRouter(directory);
        }
        finally
        {
            Directory.Delete(directory, recursive: true);
        }
        Console.WriteLine($"Mosaic XAML platform effects conformance passed ({checks} checks)");
    }
}
