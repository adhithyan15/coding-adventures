using Microsoft.UI.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using System;
using System.Runtime.InteropServices;
using System.Text.Json;
using Windows.System;
using Windows.UI.Core;

namespace Mosaic.Generated;

/// <summary>
/// Venture's package-owned adapter for the XAML project shell emitted by
/// Mosaic. Browser chrome remains generated from MIL/MLL/MSL; this adapter only
/// projects the shared Rust session and mounts its Direct2D pixels in the
/// generated content-surface slot.
/// </summary>
public static class MosaicHost
{
    private const double ViewportWidth = 1024;
    private const double ViewportHeight = 640;
    private static IntPtr browser;
    private static VentureContentSurface? contentSurface;

    public static string ApplyProps(VentureChrome component)
    {
        EnsureBrowser();
        if (browser == IntPtr.Zero)
        {
            return "Status: Venture native bridge is unavailable";
        }

        contentSurface ??= new VentureContentSurface(component);
        component.ContentSurface = contentSurface;
        var status = ApplyResponse(component, Native.Decode(Native.ApplyProps(browser)));
        contentSurface.Refresh();
        return status;
    }

    public static string HandleEvent(VentureChrome component, VentureChromeEvent ev)
    {
        EnsureBrowser();
        if (browser == IntPtr.Zero)
        {
            return "Status: Venture native bridge is unavailable";
        }

        string? value = ev is VentureChromeEvent.AddressChange addressChange
            ? addressChange.Value
            : null;
        var response = Native.Decode(Native.HandleEvent(browser, ev.MosaicName, value));
        var status = ApplyResponse(component, response);
        contentSurface?.Refresh();
        return status;
    }

    private static void EnsureBrowser()
    {
        if (browser != IntPtr.Zero)
        {
            return;
        }

        var startUrl = Environment.GetEnvironmentVariable("VENTURE_START_URL")
            ?? "http://info.cern.ch/";
        browser = Native.New(startUrl, ViewportWidth, ViewportHeight);
    }

    private static string ApplyResponse(VentureChrome component, JsonDocument? response)
    {
        using (response)
        {
            if (response is null)
            {
                return "Status: Venture native bridge returned no state";
            }

            var root = response.RootElement;
            if (!root.TryGetProperty("props", out var props))
            {
                return "Status: Venture native bridge returned malformed state";
            }

            SetIfChanged(component.Address, props.GetProperty("address").GetString(),
                value => component.Address = value);
            SetIfChanged(component.PageTitle, props.GetProperty("page-title").GetString(),
                value => component.PageTitle = value);
            SetIfChanged(component.StatusText, props.GetProperty("status-text").GetString(),
                value => component.StatusText = value);
            component.BackDisabled = props.GetProperty("back-disabled").GetBoolean();
            component.ForwardDisabled = props.GetProperty("forward-disabled").GetBoolean();
            component.NavigationDisabled = props.GetProperty("navigation-disabled").GetBoolean();

            if (root.TryGetProperty("error", out var error))
            {
                return $"Status: {error.GetString()}";
            }
            return $"Status: {component.StatusText}";
        }
    }

    private static void SetIfChanged(string current, string? next, Action<string> set)
    {
        next ??= string.Empty;
        if (!string.Equals(current, next, StringComparison.Ordinal))
        {
            set(next);
        }
    }

    private sealed class VentureContentSurface : ContentControl
    {
        private readonly VentureChrome component;
        private readonly Image image;
        private WriteableBitmap? bitmap;
        private uint pixelWidth;
        private uint pixelHeight;

        internal VentureContentSurface(VentureChrome component)
        {
            this.component = component;
            Background = new SolidColorBrush(Windows.UI.Color.FromArgb(255, 255, 255, 255));
            image = new Image { Stretch = Stretch.Fill };
            HorizontalContentAlignment = HorizontalAlignment.Stretch;
            VerticalContentAlignment = VerticalAlignment.Stretch;
            Content = image;
            IsTabStop = true;
            KeyDown += OnKeyDown;
            PointerPressed += OnPointerPressed;
            PointerWheelChanged += OnPointerWheelChanged;
            PointerReleased += OnPointerReleased;
        }

        internal void Refresh()
        {
            if (browser == IntPtr.Zero)
            {
                return;
            }

            var required = Native.RenderBgra(browser, null, 0, out var width, out var height);
            if (required == 0 || width == 0 || height == 0 || required > int.MaxValue)
            {
                return;
            }

            var pixels = new byte[(int)required];
            var written = Native.RenderBgra(browser, pixels, pixels.Length, out width, out height);
            if (written != required)
            {
                return;
            }

            if (bitmap is null || pixelWidth != width || pixelHeight != height)
            {
                bitmap = new WriteableBitmap((int)width, (int)height);
                image.Source = bitmap;
                pixelWidth = width;
                pixelHeight = height;
            }

            ((IBufferByteAccess)bitmap.PixelBuffer).Buffer(out var destination);
            Marshal.Copy(pixels, 0, destination, pixels.Length);
            bitmap.Invalidate();
        }

        private void OnPointerWheelChanged(object sender, PointerRoutedEventArgs e)
        {
            var delta = e.GetCurrentPoint(this).Properties.MouseWheelDelta;
            if (Native.Scroll(browser, -delta) != 0)
            {
                Refresh();
                e.Handled = true;
            }
        }

        private void OnPointerPressed(object sender, PointerRoutedEventArgs e)
        {
            Focus(FocusState.Pointer);
        }

        private void OnKeyDown(object sender, KeyRoutedEventArgs e)
        {
            if (e.KeyStatus.IsMenuKeyDown)
            {
                var historyEvent = e.Key switch
                {
                    VirtualKey.Left => "onBack",
                    VirtualKey.Right => "onForward",
                    _ => null,
                };
                if (historyEvent is not null)
                {
                    _ = ApplyResponse(
                        component,
                        Native.Decode(Native.HandleEvent(browser, historyEvent, null)));
                    Refresh();
                    e.Handled = true;
                    return;
                }
            }

            var shift = (InputKeyboardSource.GetKeyStateForCurrentThread(VirtualKey.Shift)
                & CoreVirtualKeyStates.Down) != 0;
            var command = e.Key switch
            {
                VirtualKey.Up => "line-up",
                VirtualKey.Down => "line-down",
                VirtualKey.PageUp => "page-up",
                VirtualKey.PageDown => "page-down",
                VirtualKey.Space when shift => "page-up",
                VirtualKey.Space => "page-down",
                VirtualKey.Home => "document-start",
                VirtualKey.End => "document-end",
                _ => null,
            };
            if (command is not null)
            {
                _ = Native.ScrollCommand(browser, command);
                Refresh();
                e.Handled = true;
            }
        }

        private void OnPointerReleased(object sender, PointerRoutedEventArgs e)
        {
            if (pixelWidth == 0 || pixelHeight == 0 || ActualWidth <= 0 || ActualHeight <= 0)
            {
                return;
            }
            var point = e.GetCurrentPoint(image).Position;
            var x = point.X * pixelWidth / ActualWidth;
            var y = point.Y * pixelHeight / ActualHeight;
            if (Native.ActivateLink(browser, x, y) != 0)
            {
                _ = ApplyProps(component);
                e.Handled = true;
            }
        }
    }

    [ComImport]
    [Guid("905a0fe0-bc53-11df-8c49-001e4fc686da")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IBufferByteAccess
    {
        void Buffer(out IntPtr value);
    }

    private static class Native
    {
        private const string Library = "venture_browser_windows";

        [DllImport(Library, EntryPoint = "venture_browser_windows_new",
            CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr New(
            [MarshalAs(UnmanagedType.LPUTF8Str)] string startUrl,
            double width,
            double height);

        [DllImport(Library, EntryPoint = "venture_browser_windows_apply_props",
            CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr ApplyProps(IntPtr browser);

        [DllImport(Library, EntryPoint = "venture_browser_windows_handle_event",
            CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr HandleEvent(
            IntPtr browser,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string name,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string? value);

        [DllImport(Library, EntryPoint = "venture_browser_windows_scroll",
            CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte Scroll(IntPtr browser, double deltaY);

        [DllImport(Library, EntryPoint = "venture_browser_windows_scroll_command",
            CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte ScrollCommand(
            IntPtr browser,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string command);

        [DllImport(Library, EntryPoint = "venture_browser_windows_activate_link",
            CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte ActivateLink(IntPtr browser, double x, double y);

        [DllImport(Library, EntryPoint = "venture_browser_windows_render_bgra",
            CallingConvention = CallingConvention.Cdecl)]
        private static extern UIntPtr RenderBgraRaw(
            IntPtr browser,
            IntPtr output,
            UIntPtr capacity,
            out uint width,
            out uint height);

        [DllImport(Library, EntryPoint = "venture_browser_windows_string_free",
            CallingConvention = CallingConvention.Cdecl)]
        private static extern void StringFree(IntPtr value);

        internal static ulong RenderBgra(
            IntPtr browser,
            byte[]? output,
            int capacity,
            out uint width,
            out uint height)
        {
            if (output is null || output.Length == 0)
            {
                return RenderBgraRaw(
                    browser,
                    IntPtr.Zero,
                    UIntPtr.Zero,
                    out width,
                    out height).ToUInt64();
            }

            var handle = GCHandle.Alloc(output, GCHandleType.Pinned);
            try
            {
                return RenderBgraRaw(
                    browser,
                    handle.AddrOfPinnedObject(),
                    (UIntPtr)(uint)Math.Max(0, capacity),
                    out width,
                    out height).ToUInt64();
            }
            finally
            {
                handle.Free();
            }
        }

        internal static JsonDocument? Decode(IntPtr value)
        {
            if (value == IntPtr.Zero)
            {
                return null;
            }
            try
            {
                var json = Marshal.PtrToStringUTF8(value);
                return json is null ? null : JsonDocument.Parse(json);
            }
            finally
            {
                StringFree(value);
            }
        }
    }
}
