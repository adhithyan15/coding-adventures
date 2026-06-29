using System;
using System.Collections;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace Mosaic.Generated;

public static class MosaicHost
{
    private const string NativeLibrary = "engram_capi";
    private static readonly object SessionLock = new();
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);
    private static IntPtr Session = Native.eg_session_new();

    static MosaicHost()
    {
        AppDomain.CurrentDomain.ProcessExit += (_, _) => FreeSession();
    }

    public static string ApplyProps(EngramApp component)
    {
        lock (SessionLock)
        {
            var json = Native.TakeString(
                Native.eg_engram_app_props(RequireSession(), CurrentDeckId(), CurrentTimeMillis()));
            return ApplyPropsFromJson(component, json, "Status: Engram host props loaded");
        }
    }

    public static string HandleEvent(EngramApp component, EngramAppEvent ev)
    {
        var envelope = JsonSerializer.Serialize(ev.MosaicEnvelope, JsonOptions);
        lock (SessionLock)
        {
            var json = Native.TakeString(
                Native.eg_handle_engram_app_event(
                    RequireSession(),
                    envelope,
                    CurrentDeckId(),
                    CurrentTimeMillis()));
            using var document = JsonDocument.Parse(json);
            var status = ApplyPropsFromRoot(
                component,
                document.RootElement,
                $"Status: Engram host handled {ev.MosaicName}");

            if (TryHostIntentSummary(document.RootElement, out var intentSummary))
            {
                return $"{status}; {intentSummary}";
            }

            return status;
        }
    }

    private static string ApplyPropsFromJson(EngramApp component, string json, string successStatus)
    {
        using var document = JsonDocument.Parse(json);
        return ApplyPropsFromRoot(component, document.RootElement, successStatus);
    }

    private static string ApplyPropsFromRoot(
        EngramApp component,
        JsonElement root,
        string successStatus)
    {
        if (root.TryGetProperty("ok", out var ok) && ok.ValueKind == JsonValueKind.False)
        {
            return $"Status: Engram host error: {JsonString(root, "error", "unknown error")}";
        }

        if (!root.TryGetProperty("props", out var props) || props.ValueKind != JsonValueKind.Object)
        {
            return successStatus;
        }

        var applied = 0;
        foreach (var prop in props.EnumerateObject())
        {
            if (TryApplySlot(component, prop.Name, prop.Value))
            {
                applied += 1;
            }
        }

        return $"{successStatus} ({applied} props)";
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

    private static bool TryHostIntentSummary(JsonElement root, out string summary)
    {
        summary = "";
        if (!root.TryGetProperty("hostIntent", out var intent) || intent.ValueKind != JsonValueKind.Object)
        {
            return false;
        }

        var type = JsonString(intent, "type", "host intent");
        summary = $"host intent: {type}";
        return true;
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

    private static IntPtr RequireSession()
    {
        if (Session == IntPtr.Zero)
        {
            throw new InvalidOperationException("Engram native session could not be created.");
        }

        return Session;
    }

    private static void FreeSession()
    {
        lock (SessionLock)
        {
            if (Session == IntPtr.Zero)
            {
                return;
            }

            Native.eg_session_free(Session);
            Session = IntPtr.Zero;
        }
    }

    private static class Native
    {
        [DllImport(NativeLibrary, EntryPoint = "eg_session_new", CallingConvention = CallingConvention.Cdecl)]
        internal static extern IntPtr eg_session_new();

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
