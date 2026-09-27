namespace CodingAdventures.X509Extension.FSharp

open System
open CodingAdventures.DerAsn1.FSharp

type X509ExtensionErrorKind =
    | Structure
    | MissingExtensionId
    | InvalidExtensionId
    | InvalidCritical
    | EncodedDefaultCritical
    | MissingExtensionValue
    | InvalidExtensionValue
    | TrailingElement

module private ErrorIds =
    let ofKind = function
        | Structure -> "structure"
        | MissingExtensionId -> "missing-extension-id"
        | InvalidExtensionId -> "invalid-extension-id"
        | InvalidCritical -> "invalid-critical"
        | EncodedDefaultCritical -> "encoded-default-critical"
        | MissingExtensionValue -> "missing-extension-value"
        | InvalidExtensionValue -> "invalid-extension-value"
        | TrailingElement -> "trailing-element"

type X509ExtensionError internal
    (kind: X509ExtensionErrorKind, offset: int, asn1Kind: Asn1ErrorKind option, framingKind: string option) =
    inherit Exception(sprintf "X.509 extension error %s at byte %d" (ErrorIds.ofKind kind) offset)
    member _.Kind = kind
    member _.KindId = ErrorIds.ofKind kind
    member _.Offset = offset
    member _.Asn1Kind = asn1Kind
    member _.FramingKind = framingKind

type X509ExtensionValue internal
    (extensionId: ObjectIdentifier, critical: bool, extensionValue: ReadOnlyMemory<byte>) =
    let snapshot = extensionValue.ToArray()
    member _.ExtensionId = extensionId
    member _.Critical = critical
    member _.ExtensionValue = ReadOnlyMemory<byte>(Array.copy snapshot)

[<RequireQualifiedAccess>]
module X509Extension =
    let private failure kind offset = X509ExtensionError(kind, offset, None, None)

    let private structure (error: Asn1Error) offset =
        X509ExtensionError(Structure, offset, Some error.Kind, error.FramingKind)

    let private semantic kind (error: Asn1Error) childOffset =
        X509ExtensionError(kind, childOffset + error.Offset, Some error.Kind, error.FramingKind)

    let private childOffset valueOffset valueLength (fields: Asn1Cursor) =
        valueOffset + valueLength - fields.Remaining.Length

    let private readChild
        (decoder: Asn1Decoder)
        (fields: Asn1Cursor)
        valueOffset
        currentChildOffset =
        try fields.Read(decoder)
        with :? Asn1Error as error ->
            let offset =
                if error.Kind = Framing then valueOffset + error.Offset
                else currentChildOffset + error.Offset
            raise (structure error offset)

    let decodeX509Extension (decoder: Asn1Decoder) (element: Asn1Element) =
        ArgumentNullException.ThrowIfNull(decoder)
        ArgumentNullException.ThrowIfNull(element)
        let valueOffset = element.Header.Length
        let valueLength = element.Value.Length
        let fields =
            try decoder.Sequence(element)
            with :? Asn1Error as error -> raise (structure error error.Offset)

        let extensionIdOffset = childOffset valueOffset valueLength fields
        let extensionIdElement =
            readChild decoder fields valueOffset extensionIdOffset
            |> Option.defaultWith (fun () -> raise (failure MissingExtensionId extensionIdOffset))
        let extensionId =
            try DerAsn1.decodeObjectIdentifierWith decoder.Limits extensionIdElement
            with :? Asn1Error as error -> raise (semantic InvalidExtensionId error extensionIdOffset)

        let secondOffset = childOffset valueOffset valueLength fields
        let second =
            readChild decoder fields valueOffset secondOffset
            |> Option.defaultWith (fun () -> raise (failure MissingExtensionValue secondOffset))

        let critical, extensionValueElement, extensionValueOffset =
            if second.Tag.Number = 1u then
                let critical =
                    try DerAsn1.decodeBoolean second
                    with :? Asn1Error as error -> raise (semantic InvalidCritical error secondOffset)
                if not critical then raise (failure EncodedDefaultCritical secondOffset)
                let valueOffset' = childOffset valueOffset valueLength fields
                let valueElement =
                    readChild decoder fields valueOffset valueOffset'
                    |> Option.defaultWith (fun () -> raise (failure MissingExtensionValue valueOffset'))
                true, valueElement, valueOffset'
            else
                false, second, secondOffset

        let extensionValue =
            try DerAsn1.decodeOctetString extensionValueElement
            with :? Asn1Error as error -> raise (semantic InvalidExtensionValue error extensionValueOffset)

        let trailingOffset = childOffset valueOffset valueLength fields
        match readChild decoder fields valueOffset trailingOffset with
        | Some _ -> raise (failure TrailingElement trailingOffset)
        | None -> X509ExtensionValue(extensionId, critical, extensionValue)
