namespace CodingAdventures.JsonValue;

using System.Collections;
using System.Globalization;
using System.Reflection;
using System.Text.Json;

public enum JsonNodeKind
{
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
}

public sealed class JsonValueError : Exception
{
    public JsonValueError(string message) : base(message)
    {
    }
}

public abstract record JsonValue
{
    public abstract JsonNodeKind Kind { get; }

    public abstract object? ToNative();

    public static JsonObjectValue JsonObject(IEnumerable<KeyValuePair<string, JsonValue>>? pairs = null)
    {
        var map = new Dictionary<string, JsonValue>(StringComparer.Ordinal);
        if (pairs is not null)
        {
            foreach (var pair in pairs)
            {
                map[pair.Key] = pair.Value;
            }
        }

        return new JsonObjectValue(map);
    }

    public static JsonArrayValue JsonArray(IEnumerable<JsonValue>? elements = null)
    {
        return new JsonArrayValue(elements?.ToList() ?? []);
    }

    public static JsonStringValue JsonString(string value) => new(value);

    public static JsonNumberValue JsonNumber(double value, bool? isInteger = null)
    {
        if (double.IsNaN(value) || double.IsInfinity(value))
        {
            throw new JsonValueError("JSON numbers cannot be NaN or infinity.");
        }

        return new JsonNumberValue(value, isInteger ?? double.IsInteger(value));
    }

    public static JsonBooleanValue JsonBool(bool value) => new(value);

    public static JsonNullValue JsonNull() => JsonNullValue.Instance;

    public static JsonValue Parse(string text)
    {
        try
        {
            using var document = JsonDocument.Parse(text);
            return FromElement(document.RootElement);
        }
        catch (JsonException exception)
        {
            throw new JsonValueError($"Failed to parse JSON: {exception.Message}");
        }
    }

    public static object? ParseNative(string text) => Parse(text).ToNative();

    public static JsonValue FromNative(object? value)
    {
        if (value is null)
        {
            return JsonNull();
        }

        if (value is JsonValue jsonValue)
        {
            return jsonValue;
        }

        switch (value)
        {
            case string stringValue:
                return JsonString(stringValue);

            case char charValue:
                return JsonString(charValue.ToString());

            case bool boolValue:
                return JsonBool(boolValue);

            case byte or sbyte or short or ushort or int or uint or long or ulong:
                return JsonNumber(Convert.ToDouble(value, CultureInfo.InvariantCulture), isInteger: true);

            case float floatValue:
                return JsonNumber(floatValue, isInteger: false);

            case double doubleValue:
                return JsonNumber(doubleValue);

            case decimal decimalValue:
                return JsonNumber((double)decimalValue, decimal.Truncate(decimalValue) == decimalValue);
        }

        if (value is IDictionary<string, object?> typedDictionary)
        {
            return JsonObject(typedDictionary.Select(pair => KeyValuePair.Create(pair.Key, FromNative(pair.Value))));
        }

        if (value is IDictionary dictionary)
        {
            var pairs = new List<KeyValuePair<string, JsonValue>>();
            foreach (DictionaryEntry entry in dictionary)
            {
                if (entry.Key is not string key)
                {
                    throw new JsonValueError("JSON object keys must be strings.");
                }

                pairs.Add(KeyValuePair.Create(key, FromNative(entry.Value)));
            }

            return JsonObject(pairs);
        }

        if (value is IEnumerable enumerable && value is not string)
        {
            var items = new List<JsonValue>();
            foreach (var item in enumerable)
            {
                items.Add(FromNative(item));
            }

            return JsonArray(items);
        }

        if (value is Delegate)
        {
            throw new JsonValueError("Delegates are not JSON-serializable.");
        }

        var type = value.GetType();
        if (type == typeof(DateTime) || type == typeof(DateTimeOffset) || type == typeof(TimeSpan) || type == typeof(Guid))
        {
            throw new JsonValueError($"Values of type '{type.Name}' are not converted implicitly. Convert them to strings first.");
        }

        if (!IsPlainObject(type))
        {
            throw new JsonValueError(
                $"Cannot convert values of type '{type.FullName}' to JsonValue. Use dictionaries, arrays, anonymous objects, or primitives.");
        }

        var reflectedPairs = type
            .GetProperties(BindingFlags.Instance | BindingFlags.Public)
            .Where(property => property.CanRead && property.GetIndexParameters().Length == 0)
            .Select(property => KeyValuePair.Create(property.Name, FromNative(property.GetValue(value))));

        return JsonObject(reflectedPairs);
    }

    internal static JsonValue FromElement(JsonElement element)
    {
        switch (element.ValueKind)
        {
            case System.Text.Json.JsonValueKind.Object:
                var objectPairs = new Dictionary<string, JsonValue>(StringComparer.Ordinal);
                foreach (var property in element.EnumerateObject())
                {
                    objectPairs[property.Name] = FromElement(property.Value);
                }

                return new JsonObjectValue(objectPairs);

            case System.Text.Json.JsonValueKind.Array:
                return new JsonArrayValue(element.EnumerateArray().Select(FromElement).ToList());

            case System.Text.Json.JsonValueKind.String:
                return new JsonStringValue(element.GetString() ?? string.Empty);

            case System.Text.Json.JsonValueKind.Number:
                var raw = element.GetRawText();
                var isInteger = !raw.Contains('.') && !raw.Contains('e') && !raw.Contains('E');
                return new JsonNumberValue(element.GetDouble(), isInteger);

            case System.Text.Json.JsonValueKind.True:
                return new JsonBooleanValue(true);

            case System.Text.Json.JsonValueKind.False:
                return new JsonBooleanValue(false);

            case System.Text.Json.JsonValueKind.Null:
                return JsonNull();

            default:
                throw new JsonValueError($"Unsupported JSON token kind '{element.ValueKind}'.");
        }
    }

    private static bool IsPlainObject(Type type)
    {
        if (type.IsPrimitive || type.IsEnum)
        {
            return false;
        }

        return type.Namespace is null
            || !type.Namespace.StartsWith("System", StringComparison.Ordinal)
            || type.IsAnonymousType();
    }
}

public sealed record JsonObjectValue(IReadOnlyDictionary<string, JsonValue> Pairs) : JsonValue
{
    public override JsonNodeKind Kind => JsonNodeKind.Object;

    public override object ToNative()
    {
        var result = new Dictionary<string, object?>(StringComparer.Ordinal);
        foreach (var pair in Pairs)
        {
            result[pair.Key] = pair.Value.ToNative();
        }

        return result;
    }
}

public sealed record JsonArrayValue(IReadOnlyList<JsonValue> Elements) : JsonValue
{
    public override JsonNodeKind Kind => JsonNodeKind.Array;

    public override object ToNative() => Elements.Select(element => element.ToNative()).ToList();
}

public sealed record JsonStringValue(string Value) : JsonValue
{
    public override JsonNodeKind Kind => JsonNodeKind.String;

    public override object ToNative() => Value;
}

public sealed record JsonNumberValue(double Value, bool IsInteger) : JsonValue
{
    public override JsonNodeKind Kind => JsonNodeKind.Number;

    public override object ToNative()
    {
        if (IsInteger
            && Value <= long.MaxValue
            && Value >= long.MinValue
            && Math.Abs(Value % 1D) < double.Epsilon)
        {
            return Convert.ToInt64(Value, CultureInfo.InvariantCulture);
        }

        return Value;
    }
}

public sealed record JsonBooleanValue(bool Value) : JsonValue
{
    public override JsonNodeKind Kind => JsonNodeKind.Boolean;

    public override object ToNative() => Value;
}

public sealed record JsonNullValue : JsonValue
{
    private JsonNullValue()
    {
    }

    public static JsonNullValue Instance { get; } = new();

    public override JsonNodeKind Kind => JsonNodeKind.Null;

    public override object? ToNative() => null;
}

internal static class TypeExtensions
{
    public static bool IsAnonymousType(this Type type)
    {
        return Attribute.IsDefined(type, typeof(System.Runtime.CompilerServices.CompilerGeneratedAttribute), false)
            && type.Name.Contains("AnonymousType", StringComparison.Ordinal)
            && type.IsGenericType;
    }
}
