// MosaicRuntimeHost.cs is compiled here too, so the adapter the library
// installs through (MosaicRuntimeHostEffects) is type-checked against the real
// generated host. Its optional color projection names Windows.UI.Color, which
// only the Windows App SDK defines; this console harness supplies the same API
// shape instead, as the runtime conformance harness beside it does. Nothing
// here loads the runtime: the checks drive the library through a fake host.
namespace Windows.UI;

public readonly record struct Color(byte A, byte R, byte G, byte B)
{
    public static Color FromArgb(byte a, byte r, byte g, byte b) => new(a, r, g, b);
}
