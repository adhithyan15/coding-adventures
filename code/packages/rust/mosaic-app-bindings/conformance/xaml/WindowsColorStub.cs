// The generated binding's color projection is compiled against the real
// Windows App SDK in the preceding TaskApp build. This console-only runtime
// harness supplies the same API shape so executing FFI conformance cannot
// initialize the hosted worker's unavailable WinUI desktop runtime.
namespace Windows.UI;

public readonly record struct Color(byte A, byte R, byte G, byte B)
{
    public static Color FromArgb(byte a, byte r, byte g, byte b) => new(a, r, g, b);
}
