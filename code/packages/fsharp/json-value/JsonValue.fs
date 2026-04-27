namespace CodingAdventures.JsonValue.FSharp

open System
open System.Collections
open System.Collections.Generic
open System.Globalization
open System.Reflection
open System.Runtime.CompilerServices
open System.Text.Json

type JsonNodeKind =
    | Object
    | Array
    | String
    | Number
    | Boolean
    | Null

type JsonValueError(message: string) =
    inherit Exception(message)

[<AbstractClass>]
type JsonValue() =
    abstract member Kind: JsonNodeKind
    abstract member ToNative: unit -> obj

    static member JsonObject(?pairs: seq<string * JsonValue>) =
        let dictionary = Dictionary<string, JsonValue>(StringComparer.Ordinal)
        for key, value in defaultArg pairs Seq.empty do
            dictionary.[key] <- value

        JsonObjectValue(dictionary) :> JsonValue

    static member JsonArray(?elements: seq<JsonValue>) =
        JsonArrayValue(defaultArg elements Seq.empty |> Seq.toList) :> JsonValue

    static member JsonString(value: string) =
        JsonStringValue(value) :> JsonValue

    static member JsonNumber(value: double, ?isInteger: bool) =
        if Double.IsNaN(value) || Double.IsInfinity(value) then
            raise (JsonValueError("JSON numbers cannot be NaN or infinity."))

        JsonNumberValue(value, defaultArg isInteger (double (Math.Truncate(value)) = value)) :> JsonValue

    static member JsonBool(value: bool) =
        JsonBooleanValue(value) :> JsonValue

    static member JsonNull() =
        JsonNullValue.Instance :> JsonValue

    static member Parse(text: string) : JsonValue =
        try
            use document = JsonDocument.Parse(text)
            JsonValue.FromElement(document.RootElement)
        with
        | :? JsonException as ex ->
            raise (JsonValueError(sprintf "Failed to parse JSON: %s" ex.Message))

    static member ParseNative(text: string) : obj =
        JsonValue.Parse(text).ToNative()

    static member FromNative(value: obj) =
        match value with
        | null ->
            JsonValue.JsonNull()
        | :? JsonValue as jsonValue ->
            jsonValue
        | :? string as stringValue ->
            JsonValue.JsonString(stringValue)
        | :? char as character ->
            JsonValue.JsonString(string character)
        | :? bool as boolValue ->
            JsonValue.JsonBool(boolValue)
        | :? byte
        | :? sbyte
        | :? int16
        | :? uint16
        | :? int
        | :? uint32
        | :? int64
        | :? uint64 as integerValue ->
            JsonValue.JsonNumber(Convert.ToDouble(integerValue, CultureInfo.InvariantCulture), true)
        | :? single as float32Value ->
            JsonValue.JsonNumber(double float32Value, false)
        | :? double as float64Value ->
            JsonValue.JsonNumber(float64Value)
        | :? decimal as decimalValue ->
            JsonValue.JsonNumber(double decimalValue, Decimal.Truncate(decimalValue) = decimalValue)
        | :? IDictionary<string, obj> as typedDictionary ->
            JsonValue.JsonObject(
                pairs =
                    (typedDictionary
                     |> Seq.map (fun pair -> pair.Key, JsonValue.FromNative(pair.Value))))
        | :? IDictionary as dictionary ->
            let pairs =
                seq {
                    for entry in dictionary do
                        let dictionaryEntry = unbox<DictionaryEntry> entry
                        match dictionaryEntry.Key with
                        | :? string as key ->
                            yield key, JsonValue.FromNative(dictionaryEntry.Value)
                        | _ ->
                            raise (JsonValueError("JSON object keys must be strings."))
                }

            JsonValue.JsonObject(pairs)
        | :? IEnumerable as enumerable when not (value :? string) ->
            JsonValue.JsonArray(elements = (enumerable |> Seq.cast<obj> |> Seq.map JsonValue.FromNative))
        | :? Delegate ->
            raise (JsonValueError("Delegates are not JSON-serializable."))
        | _ ->
            let runtimeType = value.GetType()
            if runtimeType = typeof<DateTime>
               || runtimeType = typeof<DateTimeOffset>
               || runtimeType = typeof<TimeSpan>
               || runtimeType = typeof<Guid> then
                raise (JsonValueError(sprintf "Values of type '%s' are not converted implicitly. Convert them to strings first." runtimeType.Name))

            if not (JsonValue.IsPlainObject(runtimeType)) then
                raise (JsonValueError(sprintf "Cannot convert values of type '%s' to JsonValue. Use dictionaries, arrays, anonymous objects, or primitives." runtimeType.FullName))

            JsonValue.JsonObject(
                pairs =
                    (runtimeType.GetProperties(BindingFlags.Instance ||| BindingFlags.Public)
                     |> Seq.filter (fun property -> property.CanRead && property.GetIndexParameters().Length = 0)
                     |> Seq.map (fun property -> property.Name, JsonValue.FromNative(property.GetValue(value)))))

    static member private FromElement(element: JsonElement) =
        match element.ValueKind with
        | System.Text.Json.JsonValueKind.Object ->
            JsonValue.JsonObject(
                pairs =
                    (element.EnumerateObject()
                     |> Seq.map (fun property -> property.Name, JsonValue.FromElement(property.Value))))
        | System.Text.Json.JsonValueKind.Array ->
            JsonValue.JsonArray(elements = (element.EnumerateArray() |> Seq.map JsonValue.FromElement))
        | System.Text.Json.JsonValueKind.String ->
            JsonValue.JsonString(element.GetString() |> Option.ofObj |> Option.defaultValue String.Empty)
        | System.Text.Json.JsonValueKind.Number ->
            let raw = element.GetRawText()
            let isInteger = not (raw.Contains(".")) && not (raw.Contains("e")) && not (raw.Contains("E"))
            JsonValue.JsonNumber(element.GetDouble(), isInteger)
        | System.Text.Json.JsonValueKind.True ->
            JsonValue.JsonBool(true)
        | System.Text.Json.JsonValueKind.False ->
            JsonValue.JsonBool(false)
        | System.Text.Json.JsonValueKind.Null ->
            JsonValue.JsonNull()
        | _ ->
            raise (JsonValueError(sprintf "Unsupported JSON token kind '%A'." element.ValueKind))

    static member private IsPlainObject(runtimeType: Type) =
        if runtimeType.IsPrimitive || runtimeType.IsEnum then
            false
        else
            let isAnonymous =
                Attribute.IsDefined(runtimeType, typeof<CompilerGeneratedAttribute>, false)
                && runtimeType.Name.Contains("AnonymousType", StringComparison.Ordinal)
                && runtimeType.IsGenericType

            runtimeType.Namespace = null
            || (not (runtimeType.Namespace.StartsWith("System", StringComparison.Ordinal)))
            || isAnonymous

and JsonObjectValue(pairs: IReadOnlyDictionary<string, JsonValue>) =
    inherit JsonValue()

    member _.Pairs = pairs

    override _.Kind = JsonNodeKind.Object

    override _.ToNative() =
        let dictionary = Dictionary<string, obj>(StringComparer.Ordinal)
        for KeyValue(key, value) in pairs do
            dictionary.[key] <- value.ToNative()

        box dictionary

and JsonArrayValue(elements: IReadOnlyList<JsonValue>) =
    inherit JsonValue()

    member _.Elements = elements

    override _.Kind = JsonNodeKind.Array

    override _.ToNative() =
        elements |> Seq.map (fun value -> value.ToNative()) |> ResizeArray |> box

and JsonStringValue(value: string) =
    inherit JsonValue()

    member _.Value = value

    override _.Kind = JsonNodeKind.String

    override _.ToNative() = box value

and JsonNumberValue(value: double, isInteger: bool) =
    inherit JsonValue()

    member _.Value = value
    member _.IsInteger = isInteger

    override _.Kind = JsonNodeKind.Number

    override _.ToNative() =
        if isInteger
           && value <= double Int64.MaxValue
           && value >= double Int64.MinValue
           && abs (value % 1.0) < Double.Epsilon then
            box (Convert.ToInt64(value, CultureInfo.InvariantCulture))
        else
            box value

and JsonBooleanValue(value: bool) =
    inherit JsonValue()

    member _.Value = value

    override _.Kind = JsonNodeKind.Boolean

    override _.ToNative() = box value

and JsonNullValue private () =
    inherit JsonValue()

    static let instance = JsonNullValue()

    static member Instance = instance

    override _.Kind = JsonNodeKind.Null

    override _.ToNative() = null
