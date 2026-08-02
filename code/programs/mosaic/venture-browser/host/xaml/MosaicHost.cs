using Microsoft.UI.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Automation.Peers;
using Microsoft.UI.Xaml.Automation.Provider;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Media.Imaging;
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.WindowsRuntime;
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
    private static int acceptanceReported;
    private static int interactionAcceptanceStarted;

    public static string ApplyProps(VentureChrome component)
    {
        ReportAcceptancePhase("apply-props-entered");
        EnsureBrowser();
        if (browser == IntPtr.Zero)
        {
            return "Status: Venture native bridge is unavailable";
        }
        ReportAcceptancePhase("browser-created");

        contentSurface ??= new VentureContentSurface(component);
        component.ContentSurface = contentSurface;
        ReportAcceptancePhase("content-surface-mounted");
        var status = ApplyResponse(component, Native.Decode(Native.ApplyProps(browser)));
        ReportAcceptancePhase("props-applied");
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

    public static void RunInteractionAcceptance(Window window, VentureChrome component)
    {
        var markerPath = Environment.GetEnvironmentVariable(
            "VENTURE_BROWSER_INTERACTION_ACCEPTANCE_PATH");
        var targetUrl = Environment.GetEnvironmentVariable("VENTURE_BROWSER_INTERACTION_URL");
        var startUrl = Environment.GetEnvironmentVariable("VENTURE_START_URL");
        if (string.IsNullOrWhiteSpace(markerPath)
            || string.IsNullOrWhiteSpace(targetUrl)
            || string.IsNullOrWhiteSpace(startUrl)
            || System.Threading.Interlocked.Exchange(ref interactionAcceptanceStarted, 1) != 0)
        {
            return;
        }

        component.Loaded += async (_, _) =>
        {
            try
            {
                var addressInput = await FindAutomationElementAsync<TextBox>(
                    component, "address-input");
                var backButton = await FindAutomationElementAsync<Button>(component, "back-button");
                var forwardButton = await FindAutomationElementAsync<Button>(
                    component, "forward-button");
                var homeButton = await FindAutomationElementAsync<Button>(component, "home-button");
                var reloadButton = await FindAutomationElementAsync<Button>(
                    component, "reload-button");
                var goButton = await FindAutomationElementAsync<Button>(component, "go-button");
                if (addressInput is null
                    || backButton is null
                    || forwardButton is null
                    || homeButton is null
                    || reloadButton is null
                    || goButton is null)
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native controls not found",
                    });
                    return;
                }

                var invokeProvider = new ButtonAutomationPeer(goButton) as IInvokeProvider;
                if (invokeProvider is null)
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native Go automation provider unavailable",
                    });
                    return;
                }

                _ = addressInput.Focus(FocusState.Programmatic);
                addressInput.Text = targetUrl;
                for (var remaining = 20; remaining >= 0; remaining--)
                {
                    if (string.Equals(component.Address, targetUrl, StringComparison.Ordinal))
                    {
                        break;
                    }
                    await System.Threading.Tasks.Task.Delay(50);
                }
                if (!string.Equals(component.Address, targetUrl, StringComparison.Ordinal))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        address = component.Address,
                        error = "native address edit did not dispatch",
                    });
                    return;
                }
                invokeProvider.Invoke();
                if (!await WaitForPageAsync(
                    component, targetUrl, "Venture interaction acceptance"))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        address = component.Address,
                        pageTitle = component.PageTitle,
                        error = "navigation state did not update",
                    });
                    return;
                }

                if (!await WaitForEnabledAsync(backButton))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native Back button did not become enabled",
                    });
                    return;
                }
                var backProvider = new ButtonAutomationPeer(backButton) as IInvokeProvider;
                if (backProvider is null)
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native Back automation provider unavailable",
                    });
                    return;
                }
                backProvider.Invoke();
                if (!await WaitForPageAsync(component, startUrl, "Venture launch acceptance"))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        address = component.Address,
                        pageTitle = component.PageTitle,
                        error = "back navigation state did not update",
                    });
                    return;
                }

                if (!await WaitForEnabledAsync(forwardButton))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native Forward button did not become enabled",
                    });
                    return;
                }
                var forwardProvider = new ButtonAutomationPeer(forwardButton) as IInvokeProvider;
                if (forwardProvider is null)
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native Forward automation provider unavailable",
                    });
                    return;
                }
                forwardProvider.Invoke();
                if (!await WaitForPageAsync(
                    component, targetUrl, "Venture interaction acceptance"))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        address = component.Address,
                        pageTitle = component.PageTitle,
                        error = "forward navigation state did not update",
                    });
                    return;
                }

                if (!await WaitForEnabledAsync(reloadButton))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native Reload button did not become enabled",
                    });
                    return;
                }
                var reloadProvider = new ButtonAutomationPeer(reloadButton) as IInvokeProvider;
                if (reloadProvider is null)
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native Reload automation provider unavailable",
                    });
                    return;
                }
                reloadProvider.Invoke();
                if (!await WaitForPageAsync(component, targetUrl, "Venture reload acceptance"))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        address = component.Address,
                        pageTitle = component.PageTitle,
                        error = "reload state did not update",
                    });
                    return;
                }

                if (!await WaitForEnabledAsync(homeButton))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native Home button did not become enabled",
                    });
                    return;
                }
                var homeProvider = new ButtonAutomationPeer(homeButton) as IInvokeProvider;
                if (homeProvider is null)
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native Home automation provider unavailable",
                    });
                    return;
                }
                homeProvider.Invoke();
                if (!await WaitForPageAsync(component, startUrl, "Venture launch acceptance"))
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        address = component.Address,
                        pageTitle = component.PageTitle,
                        error = "home navigation state did not update",
                    });
                    return;
                }

                if (contentSurface is null || !contentSurface.RunKeyboardAcceptance())
                {
                    WriteInteractionResult(markerPath, new
                    {
                        backend = "xaml",
                        status = "error",
                        error = "native End key did not scroll the shared viewport",
                    });
                    return;
                }

                WriteInteractionResult(markerPath, new
                {
                    backend = "xaml",
                    status = "interacted",
                    controls = "back-forward-reload-home",
                    surfaceKeyboard = "document-end",
                    reloadTitle = "Venture reload acceptance",
                    homeAddress = component.Address,
                    targetAddress = targetUrl,
                    pageTitle = component.PageTitle,
                });
            }
            catch (Exception ex)
            {
                WriteInteractionResult(markerPath, new
                {
                    backend = "xaml",
                    status = "error",
                    error = ex.ToString(),
                });
            }
        };
    }

    private static async System.Threading.Tasks.Task<bool> WaitForPageAsync(
        VentureChrome component,
        string address,
        string pageTitle)
    {
        for (var remaining = 50; remaining >= 0; remaining--)
        {
            await System.Threading.Tasks.Task.Delay(100);
            _ = ApplyProps(component);
            if (string.Equals(component.Address, address, StringComparison.Ordinal)
                && string.Equals(component.PageTitle, pageTitle, StringComparison.Ordinal))
            {
                return true;
            }
        }
        return false;
    }

    private static async System.Threading.Tasks.Task<bool> WaitForEnabledAsync(Button button)
    {
        for (var remaining = 50; remaining >= 0; remaining--)
        {
            await System.Threading.Tasks.Task.Delay(50);
            if (button.IsEnabled)
            {
                return true;
            }
        }
        return false;
    }

    private static async System.Threading.Tasks.Task<T?> FindAutomationElementAsync<T>(
        DependencyObject root,
        string automationId)
        where T : FrameworkElement
    {
        for (var remaining = 50; remaining >= 0; remaining--)
        {
            if (FindAutomationElement<T>(root, automationId) is { } element)
            {
                return element;
            }
            await System.Threading.Tasks.Task.Delay(100);
        }
        return null;
    }

    private static T? FindAutomationElement<T>(DependencyObject root, string automationId)
        where T : FrameworkElement
    {
        if (root is T candidate
            && string.Equals(
                AutomationProperties.GetAutomationId(candidate),
                automationId,
                StringComparison.Ordinal))
        {
            return candidate;
        }
        var count = VisualTreeHelper.GetChildrenCount(root);
        for (var index = 0; index < count; index++)
        {
            if (FindAutomationElement<T>(VisualTreeHelper.GetChild(root, index), automationId)
                is { } found)
            {
                return found;
            }
        }
        return null;
    }

    private static void WriteInteractionResult(string path, object result)
    {
        File.WriteAllText(path, JsonSerializer.Serialize(result) + "\n");
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
            SizeChanged += OnSizeChanged;
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
            ReportAcceptancePhase("render-size-read");
            if (required == 0 || width == 0 || height == 0 || required > int.MaxValue)
            {
                return;
            }

            var pixels = new byte[(int)required];
            var written = Native.RenderBgra(browser, pixels, pixels.Length, out width, out height);
            ReportAcceptancePhase("render-pixels-written");
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
            ReportAcceptancePhase("bitmap-allocated");

            using (var stream = bitmap.PixelBuffer.AsStream())
            {
                stream.Seek(0, SeekOrigin.Begin);
                stream.Write(pixels, 0, pixels.Length);
            }
            ReportAcceptancePhase("bitmap-copied");
            bitmap.Invalidate();
            ReportAcceptancePhase("bitmap-invalidated");
            ReportAcceptanceIfRequested();
        }

        private void OnSizeChanged(object sender, SizeChangedEventArgs e)
        {
            if (browser == IntPtr.Zero || e.NewSize.Width <= 0 || e.NewSize.Height <= 0)
            {
                return;
            }
            if (Native.Resize(browser, e.NewSize.Width, e.NewSize.Height) != 0)
            {
                Refresh();
            }
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
            var shift = (InputKeyboardSource.GetKeyStateForCurrentThread(VirtualKey.Shift)
                & CoreVirtualKeyStates.Down) != 0;
            if (HandleKey(e.Key, e.KeyStatus.IsMenuKeyDown, shift, out _))
            {
                e.Handled = true;
            }
        }

        internal bool RunKeyboardAcceptance()
        {
            _ = Focus(FocusState.Programmatic);
            return HandleKey(VirtualKey.End, false, false, out var changed) && changed;
        }

        private bool HandleKey(
            VirtualKey key,
            bool menuKeyDown,
            bool shift,
            out bool changed)
        {
            changed = false;
            if (menuKeyDown)
            {
                var historyEvent = key switch
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
                    changed = true;
                    return true;
                }
            }

            var command = key switch
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
                changed = Native.ScrollCommand(browser, command) != 0;
                Refresh();
                return true;
            }
            return false;
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

    private static void ReportAcceptanceIfRequested()
    {
        var path = Environment.GetEnvironmentVariable("VENTURE_BROWSER_ACCEPTANCE_PATH");
        if (string.IsNullOrWhiteSpace(path)
            || System.Threading.Interlocked.Exchange(ref acceptanceReported, 1) != 0)
        {
            return;
        }

        System.IO.File.WriteAllText(path, "{\"backend\":\"xaml\",\"status\":\"ready\"}\n");
    }

    private static void ReportAcceptancePhase(string phase)
    {
        var path = Environment.GetEnvironmentVariable(
            "VENTURE_BROWSER_ACCEPTANCE_DIAGNOSTIC_PATH");
        if (string.IsNullOrWhiteSpace(path))
        {
            return;
        }

        System.IO.File.WriteAllText(
            path,
            JsonSerializer.Serialize(new { backend = "xaml", phase }) + "\n");
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

        [DllImport(Library, EntryPoint = "venture_browser_windows_resize",
            CallingConvention = CallingConvention.Cdecl)]
        internal static extern byte Resize(IntPtr browser, double width, double height);

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
