namespace CodingAdventures.JsonSerializer;

using System.Globalization;
using System.Text;
using CodingAdventures.JsonValue;
using JsonNode = CodingAdventures.JsonValue.JsonValue;

public sealed class JsonSerializerError : Exception
{
    public JsonSerializerError(string message) : base(message)
    {
    }
}

public sealed record SerializerConfig(
    int IndentSize = 2,
    char IndentChar = ' ',
    bool SortKeys = false,
    bool TrailingNewline = false);

public static class JsonSerializer
{
    public static string Serialize(JsonNode value)
    {
        return value switch
        {
            JsonNullValue => "null",
            JsonBooleanValue booleanValue => booleanValue.Value ? "true" : "false",
            JsonNumberValue numberValue => SerializeNumber(numberValue),
            JsonStringValue stringValue => SerializeString(stringValue.Value),
            JsonArrayValue arrayValue => "[" + string.Join(",", arrayValue.Elements.Select(Serialize)) + "]",
            JsonObjectValue objectValue => "{" + string.Join(",", objectValue.Pairs.Select(pair => $"{SerializeString(pair.Key)}:{Serialize(pair.Value)}")) + "}",
            _ => throw new JsonSerializerError($"Unsupported JsonValue node '{value.GetType().Name}'."),
        };
    }

    public static string SerializePretty(JsonNode value, SerializerConfig? config = null)
    {
        var resolved = config ?? new SerializerConfig();
        var text = SerializePrettyRecursive(value, resolved, depth: 0);
        return resolved.TrailingNewline ? $"{text}\n" : text;
    }

    public static string Stringify(object? value) => Serialize(JsonNode.FromNative(value));

    public static string StringifyPretty(object? value, SerializerConfig? config = null)
    {
        return SerializePretty(JsonNode.FromNative(value), config);
    }

    private static string SerializePrettyRecursive(JsonNode value, SerializerConfig config, int depth)
    {
        var indentUnit = new string(config.IndentChar, config.IndentSize);
        var currentIndent = Repeat(indentUnit, depth);
        var nextIndent = Repeat(indentUnit, depth + 1);

        return value switch
        {
            JsonNullValue => "null",
            JsonBooleanValue booleanValue => booleanValue.Value ? "true" : "false",
            JsonNumberValue numberValue => SerializeNumber(numberValue),
            JsonStringValue stringValue => SerializeString(stringValue.Value),
            JsonArrayValue arrayValue => SerializePrettyArray(arrayValue, config, depth, currentIndent, nextIndent),
            JsonObjectValue objectValue => SerializePrettyObject(objectValue, config, depth, currentIndent, nextIndent),
            _ => throw new JsonSerializerError($"Unsupported JsonValue node '{value.GetType().Name}'."),
        };
    }

    private static string SerializePrettyArray(
        JsonArrayValue value,
        SerializerConfig config,
        int depth,
        string currentIndent,
        string nextIndent)
    {
        if (value.Elements.Count == 0)
        {
            return "[]";
        }

        var lines = value.Elements
            .Select(element => $"{nextIndent}{SerializePrettyRecursive(element, config, depth + 1)}");

        return $"[\n{string.Join(",\n", lines)}\n{currentIndent}]";
    }

    private static string SerializePrettyObject(
        JsonObjectValue value,
        SerializerConfig config,
        int depth,
        string currentIndent,
        string nextIndent)
    {
        if (value.Pairs.Count == 0)
        {
            return "{}";
        }

        var pairs = value.Pairs.AsEnumerable();
        if (config.SortKeys)
        {
            pairs = pairs.OrderBy(pair => pair.Key, StringComparer.Ordinal);
        }

        var lines = pairs.Select(pair =>
            $"{nextIndent}{SerializeString(pair.Key)}: {SerializePrettyRecursive(pair.Value, config, depth + 1)}");

        return $"{{\n{string.Join(",\n", lines)}\n{currentIndent}}}";
    }

    private static string SerializeNumber(JsonNumberValue number)
    {
        if (double.IsNaN(number.Value) || double.IsInfinity(number.Value))
        {
            throw new JsonSerializerError("JSON does not support NaN or infinity.");
        }

        if (number.IsInteger
            && number.Value <= long.MaxValue
            && number.Value >= long.MinValue
            && Math.Abs(number.Value % 1D) < double.Epsilon)
        {
            return Convert.ToInt64(number.Value, CultureInfo.InvariantCulture).ToString(CultureInfo.InvariantCulture);
        }

        return number.Value.ToString("R", CultureInfo.InvariantCulture);
    }

    private static string SerializeString(string value)
    {
        var builder = new StringBuilder();
        builder.Append('"');

        foreach (var ch in value)
        {
            switch (ch)
            {
                case '"':
                    builder.Append("\\\"");
                    break;
                case '\\':
                    builder.Append("\\\\");
                    break;
                case '\b':
                    builder.Append("\\b");
                    break;
                case '\f':
                    builder.Append("\\f");
                    break;
                case '\n':
                    builder.Append("\\n");
                    break;
                case '\r':
                    builder.Append("\\r");
                    break;
                case '\t':
                    builder.Append("\\t");
                    break;
                default:
                    if (char.IsControl(ch))
                    {
                        builder.Append("\\u");
                        builder.Append(((int)ch).ToString("x4", CultureInfo.InvariantCulture));
                    }
                    else
                    {
                        builder.Append(ch);
                    }

                    break;
            }
        }

        builder.Append('"');
        return builder.ToString();
    }

    private static string Repeat(string text, int count)
    {
        var builder = new StringBuilder();
        for (var index = 0; index < count; index++)
        {
            builder.Append(text);
        }

        return builder.ToString();
    }
}
