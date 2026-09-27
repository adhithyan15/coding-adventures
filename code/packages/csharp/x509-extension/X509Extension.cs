using System.Globalization;
using Asn1Api = CodingAdventures.DerAsn1.CSharp.DerAsn1;

namespace CodingAdventures.X509Extension.CSharp;

public enum X509ExtensionErrorKind
{
    Structure,
    MissingExtensionId,
    InvalidExtensionId,
    InvalidCritical,
    EncodedDefaultCritical,
    MissingExtensionValue,
    InvalidExtensionValue,
    TrailingElement,
}

public sealed class X509ExtensionError : Exception
{
    internal X509ExtensionError(
        X509ExtensionErrorKind kind,
        int offset,
        Asn1Api.Asn1ErrorKind? asn1Kind = null,
        string? framingKind = null)
        : base($"X.509 extension error {ErrorId(kind)} at byte {offset.ToString(CultureInfo.InvariantCulture)}")
    {
        Kind = kind;
        Offset = offset;
        Asn1Kind = asn1Kind;
        FramingKind = framingKind;
    }

    public X509ExtensionErrorKind Kind { get; }
    public string KindId => ErrorId(Kind);
    public int Offset { get; }
    public Asn1Api.Asn1ErrorKind? Asn1Kind { get; }
    public string? FramingKind { get; }

    internal static string ErrorId(X509ExtensionErrorKind kind) => kind switch
    {
        X509ExtensionErrorKind.Structure => "structure",
        X509ExtensionErrorKind.MissingExtensionId => "missing-extension-id",
        X509ExtensionErrorKind.InvalidExtensionId => "invalid-extension-id",
        X509ExtensionErrorKind.InvalidCritical => "invalid-critical",
        X509ExtensionErrorKind.EncodedDefaultCritical => "encoded-default-critical",
        X509ExtensionErrorKind.MissingExtensionValue => "missing-extension-value",
        X509ExtensionErrorKind.InvalidExtensionValue => "invalid-extension-value",
        X509ExtensionErrorKind.TrailingElement => "trailing-element",
        _ => throw new ArgumentOutOfRangeException(nameof(kind)),
    };
}

public sealed class X509ExtensionValue
{
    private readonly byte[] extensionValue;

    internal X509ExtensionValue(
        Asn1Api.ObjectIdentifier extensionId,
        bool critical,
        ReadOnlyMemory<byte> extensionValue)
    {
        ExtensionId = extensionId;
        Critical = critical;
        this.extensionValue = extensionValue.ToArray();
    }

    public Asn1Api.ObjectIdentifier ExtensionId { get; }
    public bool Critical { get; }
    public ReadOnlyMemory<byte> ExtensionValue => extensionValue.ToArray();
}

public static class X509Extension
{
    public static X509ExtensionValue DecodeX509Extension(
        Asn1Api.Asn1Decoder decoder,
        Asn1Api.Asn1Element element)
    {
        ArgumentNullException.ThrowIfNull(decoder);
        ArgumentNullException.ThrowIfNull(element);
        int valueOffset = element.Header.Length;
        int valueLength = element.Value.Length;
        Asn1Api.Asn1Cursor fields;
        try
        {
            fields = decoder.Sequence(element);
        }
        catch (Asn1Api.Error error)
        {
            throw Structure(error, error.Offset);
        }

        int extensionIdOffset = ChildOffset(valueOffset, valueLength, fields);
        Asn1Api.Asn1Element? extensionIdElement = ReadChild(
            decoder, fields, valueOffset, extensionIdOffset);
        if (extensionIdElement is null)
            throw Failure(X509ExtensionErrorKind.MissingExtensionId, extensionIdOffset);

        Asn1Api.ObjectIdentifier extensionId;
        try
        {
            extensionId = Asn1Api.DecodeObjectIdentifier(extensionIdElement, decoder.Limits);
        }
        catch (Asn1Api.Error error)
        {
            throw Semantic(X509ExtensionErrorKind.InvalidExtensionId, error, extensionIdOffset);
        }

        int secondOffset = ChildOffset(valueOffset, valueLength, fields);
        Asn1Api.Asn1Element? second = ReadChild(decoder, fields, valueOffset, secondOffset);
        if (second is null)
            throw Failure(X509ExtensionErrorKind.MissingExtensionValue, secondOffset);

        bool critical;
        Asn1Api.Asn1Element extensionValueElement;
        int extensionValueOffset;
        if (second.Tag.Number == 1)
        {
            try
            {
                critical = Asn1Api.DecodeBoolean(second);
            }
            catch (Asn1Api.Error error)
            {
                throw Semantic(X509ExtensionErrorKind.InvalidCritical, error, secondOffset);
            }
            if (!critical)
                throw Failure(X509ExtensionErrorKind.EncodedDefaultCritical, secondOffset);
            extensionValueOffset = ChildOffset(valueOffset, valueLength, fields);
            extensionValueElement = ReadChild(decoder, fields, valueOffset, extensionValueOffset)
                ?? throw Failure(X509ExtensionErrorKind.MissingExtensionValue, extensionValueOffset);
        }
        else
        {
            critical = false;
            extensionValueElement = second;
            extensionValueOffset = secondOffset;
        }

        ReadOnlyMemory<byte> extensionValue;
        try
        {
            extensionValue = Asn1Api.DecodeOctetString(extensionValueElement);
        }
        catch (Asn1Api.Error error)
        {
            throw Semantic(X509ExtensionErrorKind.InvalidExtensionValue, error, extensionValueOffset);
        }

        int trailingOffset = ChildOffset(valueOffset, valueLength, fields);
        if (ReadChild(decoder, fields, valueOffset, trailingOffset) is not null)
            throw Failure(X509ExtensionErrorKind.TrailingElement, trailingOffset);
        return new X509ExtensionValue(extensionId, critical, extensionValue);
    }

    private static Asn1Api.Asn1Element? ReadChild(
        Asn1Api.Asn1Decoder decoder,
        Asn1Api.Asn1Cursor fields,
        int valueOffset,
        int childOffset)
    {
        try
        {
            return fields.Read(decoder);
        }
        catch (Asn1Api.Error error)
        {
            int offset = error.Kind == Asn1Api.Asn1ErrorKind.Framing
                ? valueOffset + error.Offset
                : childOffset + error.Offset;
            throw Structure(error, offset);
        }
    }

    private static int ChildOffset(int valueOffset, int valueLength, Asn1Api.Asn1Cursor fields) =>
        valueOffset + valueLength - fields.Remaining.Length;

    private static X509ExtensionError Semantic(
        X509ExtensionErrorKind kind,
        Asn1Api.Error error,
        int childOffset) =>
        new(kind, childOffset + error.Offset, error.Kind, error.FramingKind);

    private static X509ExtensionError Structure(Asn1Api.Error error, int offset) =>
        new(X509ExtensionErrorKind.Structure, offset, error.Kind, error.FramingKind);

    private static X509ExtensionError Failure(X509ExtensionErrorKind kind, int offset) =>
        new(kind, offset);
}
