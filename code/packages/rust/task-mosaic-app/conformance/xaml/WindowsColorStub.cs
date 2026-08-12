// The generated binding is compiled against the real Windows App SDK in the
// preceding TaskApp build. This console-only runtime harness supplies the same
// API shape so FFI conformance does not initialize a WinUI desktop.
namespace Windows.UI;

public readonly record struct Color(byte A, byte R, byte G, byte B)
{
    public static Color FromArgb(byte a, byte r, byte g, byte b) => new(a, r, g, b);
}
