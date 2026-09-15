// PhotoPickerEffects.cs — the XAML handler for UI59's `files.open` effect.
//
// See code/specs/UI59-files-open-effect.md for the full contract this
// implements. Declared via `[host_effects]` in mosaic-package.toml;
// `Install()` matches the XAML install signature `UI47` §5.5.3 specifies
// exactly: `static void Install()`, no arguments, setting
// `MosaicRuntimeHost.EffectHandler`.

using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Threading.Tasks;
using Mosaic.Generated; // MosaicRuntimeHost -- generated into this namespace.
using Windows.Storage;
using Windows.Storage.Pickers;
using WinRT.Interop;

// Deliberately NOT `namespace PhotoPickerApp` -- the generated component
// class itself is `Mosaic.Generated.PhotoPickerApp` (from the .mil's
// `component PhotoPickerApp`), and MainWindow.xaml.cs's generated
// `PhotoPickerApp.PhotoPickerEffects.Install();` call site lives inside
// `namespace Mosaic.Generated`, where an unqualified `PhotoPickerApp`
// resolves to that class, not this namespace -- confirmed by a real build
// failure (CS0117: 'PhotoPickerApp' does not contain a definition for
// 'PhotoPickerEffects') before this rename. `PhotoPickerHost` avoids the
// collision; `mosaic-package.toml`'s `install` value matches.
namespace PhotoPickerHost;

public static class PhotoPickerEffects
{
    // Common photo MIME types this app's own request asks for (see
    // photo-picker-mosaic-app's ACCEPT_IMAGE_TYPES) mapped to the file
    // extensions WinUI 3's FileTypeFilter actually wants -- FileOpenPicker
    // filters by extension, not MIME type, so the handler owns this table
    // rather than the app needing to know XAML-specific filter syntax
    // (UI59 §3).
    private static readonly Dictionary<string, string[]> MimeTypeExtensions = new()
    {
        ["image/jpeg"] = new[] { ".jpg", ".jpeg" },
        ["image/png"] = new[] { ".png" },
        ["image/webp"] = new[] { ".webp" },
        ["image/gif"] = new[] { ".gif" },
        ["image/bmp"] = new[] { ".bmp" },
        ["image/tiff"] = new[] { ".tif", ".tiff" },
    };

    // The reverse of the table above, for reporting the picked file's MIME
    // type back to the app (UI59 §3's `mimeType` result field) -- a
    // FileOpenPicker result carries a path, not a MIME type, so this is
    // recovered from the extension.
    private static readonly Dictionary<string, string> ExtensionMimeTypes = BuildExtensionMimeTypes();

    private static Dictionary<string, string> BuildExtensionMimeTypes()
    {
        var map = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var (mimeType, extensions) in MimeTypeExtensions)
        {
            foreach (var extension in extensions)
            {
                map[extension] = mimeType;
            }
        }
        return map;
    }

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    public static void Install()
    {
        MosaicRuntimeHost.EffectHandler = (id, kind, payload, delivery) =>
        {
            if (!string.Equals(delivery, "await", StringComparison.OrdinalIgnoreCase))
            {
                return;
            }
            if (kind != "files.open")
            {
                return;
            }
            if (!MosaicRuntimeHost.DeferEffect(id))
            {
                // Another handler already claimed this effect, or it's not
                // actually pending -- nothing for this handler to do.
                return;
            }
            // EffectHandler is a synchronous Action; PickSingleFileAsync is
            // necessarily async (it waits on user interaction, which can
            // take an unbounded amount of time). DeferEffect above took
            // ownership of completing this effect later; fire the async
            // continuation without awaiting it here (UI59 §4.2).
            _ = PickAndCompleteAsync(id, payload);
        };
    }

    private static async Task PickAndCompleteAsync(ulong id, JsonElement payload)
    {
        try
        {
            var picker = new FileOpenPicker
            {
                SuggestedStartLocation = PickerLocationId.PicturesLibrary,
            };

            var anyRecognisedFilter = false;
            if (payload.ValueKind == JsonValueKind.Object &&
                payload.TryGetProperty("accept", out var accept) &&
                accept.ValueKind == JsonValueKind.Array)
            {
                foreach (var mimeTypeElement in accept.EnumerateArray())
                {
                    // Named `candidateMimeType`, not `mimeType`: C#'s local
                    // scoping is lexical (an enclosing method scope, not
                    // runtime overlap), so this loop-local name would
                    // collide with the unrelated `mimeType` declared later
                    // in this same method (the picked file's resolved
                    // type) even though their lifetimes never overlap --
                    // hit as a real CS0136 build error before this rename.
                    var candidateMimeType = mimeTypeElement.GetString();
                    if (candidateMimeType is null ||
                        !MimeTypeExtensions.TryGetValue(candidateMimeType, out var extensions))
                    {
                        // An unrecognised MIME type is dropped from the
                        // filter rather than failing the request (UI59
                        // §3) -- a caller asking for a type this host
                        // doesn't know how to filter for should still get
                        // a working picker.
                        continue;
                    }
                    foreach (var extension in extensions)
                    {
                        picker.FileTypeFilter.Add(extension);
                    }
                    anyRecognisedFilter = true;
                }
            }
            if (!anyRecognisedFilter)
            {
                // No `accept`, an empty one, or nothing this host
                // recognised: "any file" (UI59 §3). FileTypeFilter must
                // have at least one entry or FileOpenPicker throws.
                picker.FileTypeFilter.Add("*");
            }

            // FileOpenPicker needs an owner HWND (InitializeWithWindow) on
            // an unpackaged WinUI 3 app -- confirmed against the legacy
            // Anki-import picker in engram-app's pre-UI47 XAML host, the
            // one other real FileOpenPicker usage in this repo. `Install()`
            // itself receives no window reference (the `[host_effects]`
            // contract doesn't provide one -- it's called from inside
            // MainWindow's own constructor, before the window is even
            // fully built), and the generated App/MainWindow expose no
            // static accessor for it either. GetForegroundWindow() at the
            // moment the picker actually opens (well after startup, in
            // response to a real user click) is a standard, working
            // technique for exactly this situation: the app's own window
            // is definitionally the foreground window right after the
            // user just clicked a button in it. A cleaner fix would have
            // the generator expose a static window accessor; that's
            // follow-up work for the emitter, not this handler.
            var ownerHandle = GetForegroundWindow();
            InitializeWithWindow.Initialize(picker, ownerHandle);

            StorageFile? file = await picker.PickSingleFileAsync();
            if (file is null)
            {
                MosaicRuntimeHost.CompleteEffect(id, new { cancelled = new { } });
                return;
            }

            var bytes = await ReadAllBytesAsync(file);
            var mimeType = ExtensionMimeTypes.TryGetValue(file.FileType, out var known)
                ? known
                : "application/octet-stream";
            MosaicRuntimeHost.CompleteEffect(id, new
            {
                ok = new
                {
                    name = file.Name,
                    mimeType,
                    bytes = Convert.ToBase64String(bytes),
                },
            });
        }
        catch (Exception error)
        {
            MosaicRuntimeHost.CompleteEffect(id, new { failed = new { message = error.Message } });
        }
    }

    private static async Task<byte[]> ReadAllBytesAsync(StorageFile file)
    {
        using var stream = await file.OpenStreamForReadAsync();
        using var memory = new MemoryStream();
        await stream.CopyToAsync(memory);
        return memory.ToArray();
    }
}
