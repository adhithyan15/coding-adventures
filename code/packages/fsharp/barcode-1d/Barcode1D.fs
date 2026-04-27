namespace CodingAdventures.Barcode1D.FSharp

open System
open CodingAdventures.BarcodeLayout1D.FSharp
open CodingAdventures.PaintInstructions
open CodingAdventures.PixelContainer

[<RequireQualifiedAccess>]
type Symbology =
    | Codabar
    | Code128
    | Code39
    | Ean13
    | Itf
    | UpcA

type Barcode1DOptions =
    {
        Symbology: Symbology
        Paint: PaintBarcode1DOptions
        CodabarStart: string option
        CodabarStop: string option
    }

type Barcode1DError(message: string) =
    inherit Exception(message)

type UnsupportedSymbologyException(message: string) =
    inherit Barcode1DError(message)

type BackendUnavailableException(message: string) =
    inherit Barcode1DError(message)

[<RequireQualifiedAccess>]
module Barcode1D =
    [<Literal>]
    let VERSION = "0.1.0"

    let defaultRenderConfig =
        BarcodeLayout1D.defaultRenderConfig

    let defaultOptions =
        {
            Symbology = Symbology.Code39
            Paint = BarcodeLayout1D.defaultPaintOptions
            CodabarStart = None
            CodabarStop = None
        }

    let symbologyAsString symbology =
        match symbology with
        | Symbology.Codabar -> "codabar"
        | Symbology.Code128 -> "code128"
        | Symbology.Code39 -> "code39"
        | Symbology.Ean13 -> "ean13"
        | Symbology.Itf -> "itf"
        | Symbology.UpcA -> "upca"

    let currentBackend () : string option =
        None

    let normalizeSymbology (symbology: string) =
        if isNull symbology then
            nullArg (nameof symbology)

        let normalized =
            symbology.Trim().ToLowerInvariant().Replace("-", String.Empty).Replace("_", String.Empty)

        let normalized =
            if normalized.Length = 0 then "code39" else normalized

        match normalized with
        | "codabar" -> Symbology.Codabar
        | "code128" -> Symbology.Code128
        | "code39" -> Symbology.Code39
        | "ean13" -> Symbology.Ean13
        | "itf" -> Symbology.Itf
        | "upca" -> Symbology.UpcA
        | _ -> raise (UnsupportedSymbologyException $"unsupported symbology: {symbology}")

    let buildScene data options =
        let options = defaultArg options defaultOptions

        match options.Symbology with
        | Symbology.Codabar ->
            CodingAdventures.Codabar.FSharp.Codabar.layoutCodabar
                data
                (Some options.Paint)
                options.CodabarStart
                options.CodabarStop
        | Symbology.Code128 ->
            CodingAdventures.Code128.FSharp.Code128.layoutCode128 data (Some options.Paint)
        | Symbology.Code39 ->
            CodingAdventures.Code39.FSharp.Code39.layoutCode39 data (Some options.Paint)
        | Symbology.Ean13 ->
            CodingAdventures.Ean13.FSharp.Ean13.layoutEan13 data (Some options.Paint)
        | Symbology.Itf ->
            CodingAdventures.Itf.FSharp.Itf.layoutItf data (Some options.Paint)
        | Symbology.UpcA ->
            CodingAdventures.UpcA.FSharp.UpcA.layoutUpcA data (Some options.Paint)

    let buildSceneForSymbology symbology data options =
        let options =
            { defaultArg options defaultOptions with
                Symbology = normalizeSymbology symbology }

        buildScene data (Some options)

    let private backendUnavailable () =
        raise (
            BackendUnavailableException
                "native barcode rendering is not wired for dotnet yet; buildScene is available, but pixel and PNG rendering await a paint backend."
        )

    let renderPixels data options : PixelContainer =
        buildScene data options |> ignore
        backendUnavailable ()

    let renderPixelsForSymbology symbology data options : PixelContainer =
        buildSceneForSymbology symbology data options |> ignore
        backendUnavailable ()

    let renderPng data options : byte array =
        renderPixels data options |> ignore
        backendUnavailable ()

    let renderPngForSymbology symbology data options : byte array =
        renderPixelsForSymbology symbology data options |> ignore
        backendUnavailable ()
