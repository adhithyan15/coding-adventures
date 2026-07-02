using System;
using System.Collections;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace Mosaic.Generated;

public sealed record MosaicHostIntent(string Type, string Json);

public sealed record MosaicHostResult(string Status, MosaicHostIntent? HostIntent = null);

public static class MosaicHost
{
    private const string NativeLibrary = "engram_capi";
    private static readonly object SessionLock = new();
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);
    private static IntPtr Session = IntPtr.Zero;
    private static string? LoadError;

    public static event EventHandler<MosaicHostIntent>? HostIntentReceived;

    public static MosaicHostIntent? LastHostIntent { get; private set; }

    static MosaicHost()
    {
        AppDomain.CurrentDomain.ProcessExit += (_, _) => FreeSession();
    }

    public static MosaicHostResult ApplyProps(EngramApp component)
    {
        lock (SessionLock)
        {
            if (!TryGetSession(out var session, out var unavailable))
            {
                return HostUnavailable(unavailable);
            }

            try
            {
                var json = Native.TakeString(
                    Native.eg_engram_app_props(session, CurrentDeckId(), CurrentTimeMillis()));
                return ApplyPropsFromJson(component, json, "Status: Engram host props loaded");
            }
            catch (Exception ex) when (IsNativeAvailabilityFailure(ex))
            {
                LoadError = Describe(ex);
                Session = IntPtr.Zero;
                return HostUnavailable(LoadError);
            }
        }
    }

    public static MosaicHostResult HandleEvent(EngramApp component, EngramAppEvent ev)
    {
        var envelope = JsonSerializer.Serialize(ev.MosaicEnvelope, JsonOptions);
        lock (SessionLock)
        {
            if (!TryGetSession(out var session, out var unavailable))
            {
                return HostUnavailable(unavailable);
            }

            try
            {
                var json = Native.TakeString(
                    Native.eg_handle_engram_app_event(
                        session,
                        envelope,
                        CurrentDeckId(),
                        CurrentTimeMillis()));
                using var document = JsonDocument.Parse(json);
                return ApplyPropsFromRoot(
                    component,
                    document.RootElement,
                    $"Status: Engram host handled {ev.MosaicName}");
            }
            catch (Exception ex) when (IsNativeAvailabilityFailure(ex))
            {
                LoadError = Describe(ex);
                Session = IntPtr.Zero;
                return HostUnavailable(LoadError);
            }
        }
    }

    private static MosaicHostResult ApplyPropsFromJson(
        EngramApp component,
        string json,
        string successStatus)
    {
        using var document = JsonDocument.Parse(json);
        return ApplyPropsFromRoot(component, document.RootElement, successStatus);
    }

    private static MosaicHostResult ApplyPropsFromRoot(
        EngramApp component,
        JsonElement root,
        string successStatus)
    {
        if (root.TryGetProperty("ok", out var ok) && ok.ValueKind == JsonValueKind.False)
        {
            return new MosaicHostResult(
                $"Status: Engram host error: {JsonString(root, "error", "unknown error")}");
        }

        var hostIntent = CaptureHostIntent(root);
        if (!root.TryGetProperty("props", out var props) || props.ValueKind != JsonValueKind.Object)
        {
            return WithHostIntentStatus(successStatus, hostIntent);
        }

        var applied = 0;
        foreach (var prop in props.EnumerateObject())
        {
            if (TryApplySlot(component, prop.Name, prop.Value))
            {
                applied += 1;
            }
        }

        return WithHostIntentStatus($"{successStatus} ({applied} props)", hostIntent);
    }

    private static bool TryApplySlot(EngramApp component, string slotName, JsonElement value)
    {
        var propertyName = SlotNameToPropertyName(slotName);
        var property = typeof(EngramApp).GetProperty(
            propertyName,
            BindingFlags.Instance | BindingFlags.Public);
        if (property is null || !property.CanWrite)
        {
            return false;
        }

        var converted = ConvertJsonValue(value, property.PropertyType);
        property.SetValue(component, converted);
        return true;
    }

    private static object? ConvertJsonValue(JsonElement value, Type targetType)
    {
        if (targetType == typeof(string))
        {
            return value.ValueKind switch
            {
                JsonValueKind.Null => "",
                JsonValueKind.String => value.GetString() ?? "",
                _ => value.ToString(),
            };
        }

        if (targetType == typeof(double))
        {
            return value.ValueKind == JsonValueKind.Number
                ? value.GetDouble()
                : double.TryParse(value.ToString(), out var parsed) ? parsed : 0.0;
        }

        if (targetType == typeof(bool))
        {
            return value.ValueKind == JsonValueKind.True
                || (value.ValueKind == JsonValueKind.String
                    && bool.TryParse(value.GetString(), out var parsed)
                    && parsed);
        }

        if (targetType.IsGenericType
            && targetType.GetGenericTypeDefinition() == typeof(IReadOnlyList<>)
            && value.ValueKind == JsonValueKind.Array)
        {
            var elementType = targetType.GetGenericArguments()[0];
            var listType = typeof(List<>).MakeGenericType(elementType);
            var list = (IList)Activator.CreateInstance(listType)!;
            foreach (var item in value.EnumerateArray())
            {
                list.Add(ConvertJsonValue(item, elementType));
            }
            return list;
        }

        return JsonSerializer.Deserialize(value.GetRawText(), targetType, JsonOptions);
    }

    private static MosaicHostResult WithHostIntentStatus(
        string status,
        MosaicHostIntent? hostIntent)
    {
        return hostIntent is null
            ? new MosaicHostResult(status)
            : new MosaicHostResult($"{status}; host intent: {hostIntent.Type}", hostIntent);
    }

    private static MosaicHostIntent? CaptureHostIntent(JsonElement root)
    {
        if (!root.TryGetProperty("hostIntent", out var intent) || intent.ValueKind != JsonValueKind.Object)
        {
            return null;
        }

        var type = JsonString(intent, "type", "host intent");
        var hostIntent = new MosaicHostIntent(type, intent.GetRawText());
        LastHostIntent = hostIntent;
        HostIntentReceived?.Invoke(null, hostIntent);
        return hostIntent;
    }

    private static string JsonString(JsonElement root, string property, string fallback)
    {
        return root.TryGetProperty(property, out var value) && value.ValueKind == JsonValueKind.String
            ? value.GetString() ?? fallback
            : fallback;
    }

    private static string SlotNameToPropertyName(string slotName)
    {
        var builder = new StringBuilder(slotName.Length);
        var uppercaseNext = true;
        foreach (var ch in slotName)
        {
            if (ch is '-' or '_' or ' ')
            {
                uppercaseNext = true;
                continue;
            }

            builder.Append(uppercaseNext ? char.ToUpperInvariant(ch) : ch);
            uppercaseNext = false;
        }

        return builder.ToString();
    }

    private static string CurrentDeckId()
    {
        return Environment.GetEnvironmentVariable("ENGRAM_DECK_ID") ?? "";
    }

    private static ulong CurrentTimeMillis()
    {
        return (ulong)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
    }

    private static bool TryGetSession(out IntPtr session, out string unavailable)
    {
        if (Session != IntPtr.Zero)
        {
            session = Session;
            unavailable = "";
            return true;
        }

        if (LoadError is not null)
        {
            session = IntPtr.Zero;
            unavailable = LoadError;
            return false;
        }

        try
        {
            Session = Native.eg_session_new_demo();
        }
        catch (Exception ex) when (IsNativeAvailabilityFailure(ex))
        {
            LoadError = Describe(ex);
            session = IntPtr.Zero;
            unavailable = LoadError;
            return false;
        }

        if (Session == IntPtr.Zero)
        {
            LoadError = "eg_session_new_demo returned null";
            session = IntPtr.Zero;
            unavailable = LoadError;
            return false;
        }

        session = Session;
        unavailable = "";
        return true;
    }

    private static MosaicHostResult HostUnavailable(string reason)
    {
        return new MosaicHostResult($"Status: Engram native host unavailable: {reason}");
    }

    private static bool IsNativeAvailabilityFailure(Exception ex)
    {
        return ex is DllNotFoundException
            or EntryPointNotFoundException
            or BadImageFormatException
            or MarshalDirectiveException;
    }

    private static string Describe(Exception ex)
    {
        return $"{ex.GetType().Name}: {ex.Message}";
    }

    private static void FreeSession()
    {
        lock (SessionLock)
        {
            if (Session == IntPtr.Zero)
            {
                return;
            }

            try
            {
                Native.eg_session_free(Session);
            }
            catch (Exception ex) when (IsNativeAvailabilityFailure(ex))
            {
                LoadError = Describe(ex);
            }
            Session = IntPtr.Zero;
        }
    }

    private static class Native
    {
        [DllImport(NativeLibrary, EntryPoint = "eg_session_new_demo", CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr eg_session_new_demo();

        [DllImport(NativeLibrary, EntryPoint = "eg_session_free", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void eg_session_free(IntPtr session);

        [DllImport(NativeLibrary, EntryPoint = "eg_string_free", CallingConvention = CallingConvention.Cdecl)]
        internal static extern void eg_string_free(IntPtr value);

        [DllImport(NativeLibrary, EntryPoint = "eg_engram_app_props", CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr eg_engram_app_props(
            IntPtr session,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string deckId,
            ulong now);

        [DllImport(NativeLibrary, EntryPoint = "eg_handle_engram_app_event", CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr eg_handle_engram_app_event(
            IntPtr session,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string eventJson,
            [MarshalAs(UnmanagedType.LPUTF8Str)] string deckId,
            ulong now);

        internal static string TakeString(IntPtr value)
        {
            if (value == IntPtr.Zero)
            {
                return "{\"ok\":false,\"error\":\"Engram native host returned null\"}";
            }

            try
            {
                return Marshal.PtrToStringUTF8(value) ?? "";
            }
            finally
            {
                eg_string_free(value);
            }
        }
    }
}
