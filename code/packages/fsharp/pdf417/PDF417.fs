namespace CodingAdventures.PDF417.FSharp

open System
open CodingAdventures.Barcode2D

/// Options controlling how the PDF417 symbol is encoded.
type PDF417Options =
    {
        /// Reed-Solomon error correction level, 0 through 8. None selects automatically.
        EccLevel: int option
        /// Number of data columns, 1 through 30. None selects automatically.
        Columns: int option
        /// Module-rows per logical PDF417 row. Values below 1 are clamped by the engine.
        RowHeight: int
    }

/// F# facade for the shared .NET PDF417 encoder.
[<RequireQualifiedAccess>]
module PDF417 =
    /// Package version.
    [<Literal>]
    let VERSION = "0.1.0"

    /// Default PDF417 encoding options.
    let defaultOptions =
        { EccLevel = None; Columns = None; RowHeight = 3 }

    let private toEngineOptions (options: PDF417Options option) =
        let opts = defaultArg options defaultOptions
        CodingAdventures.PDF417.PDF417Options(
            EccLevel = Option.toNullable opts.EccLevel,
            Columns = Option.toNullable opts.Columns,
            RowHeight = opts.RowHeight)

    /// Encode raw bytes as a PDF417 symbol.
    let encodeBytes (data: byte[]) (options: PDF417Options option) : ModuleGrid =
        if isNull data then
            nullArg "data"

        CodingAdventures.PDF417.PDF417Encoder.Encode(data, toEngineOptions options)

    /// Encode UTF-8 text as a PDF417 symbol.
    let encodeText (text: string) (options: PDF417Options option) : ModuleGrid =
        if isNull text then
            nullArg "text"

        CodingAdventures.PDF417.PDF417Encoder.Encode(text, toEngineOptions options)

    /// Encode text and preserve the C# package's default option behavior.
    let encode (text: string) : ModuleGrid =
        encodeText text None
