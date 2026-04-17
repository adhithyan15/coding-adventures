import Foundation
import BarcodeLayout1D
import Codabar
import Code128
import Code39
import EAN13
import ITF
import PaintCodecPNGNative
import PaintInstructions
#if os(macOS) && arch(arm64)
import PaintVmMetalNative
#elseif os(Windows)
import PaintVmDirect2DNative
#endif
import PixelContainer
import UPCA

public enum Barcode1DError: Error, Equatable {
    case unsupportedSymbology(String)
    case backendUnavailable
}

public enum Barcode1D {
    public static let defaultLayoutConfig = defaultCode39LayoutConfig
    public static let defaultRenderConfig = defaultLayoutConfig

    public static func currentBackend() -> String? {
        #if os(macOS) && arch(arm64)
        return "metal"
        #elseif os(Windows)
        return "direct2d"
        #else
        return nil
        #endif
    }

    private static func normalizeSymbology(_ symbology: String) throws -> String {
        let normalized = symbology.replacingOccurrences(of: "-", with: "")
            .replacingOccurrences(of: "_", with: "")
            .lowercased()
        if normalized == "codabar" {
            return normalized
        }
        if normalized == "code128" {
            return normalized
        }
        if normalized == "code39" {
            return normalized
        }
        if normalized == "ean13" {
            return normalized
        }
        if normalized == "itf" {
            return normalized
        }
        if normalized == "upca" {
            return normalized
        }
        throw Barcode1DError.unsupportedSymbology(symbology)
    }

    public static func buildScene(
        _ data: String,
        symbology: String = "code39",
        layoutConfig: Barcode1DLayoutConfig = defaultLayoutConfig
    ) throws -> PaintScene {
        switch try normalizeSymbology(symbology) {
        case "codabar":
            return try layoutCodabar(data, config: layoutConfig)
        case "code128":
            return try layoutCode128(data, config: layoutConfig)
        case "code39":
            return try layoutCode39(data, config: layoutConfig)
        case "ean13":
            return try layoutEAN13(data, config: layoutConfig)
        case "itf":
            return try layoutITF(data, config: layoutConfig)
        case "upca":
            return try layoutUPCA(data, config: layoutConfig)
        default:
            throw Barcode1DError.unsupportedSymbology(symbology)
        }
    }

    public static func renderPixels(
        _ data: String,
        symbology: String = "code39",
        layoutConfig: Barcode1DLayoutConfig = defaultLayoutConfig
    ) throws -> PixelContainer {
        let scene = try buildScene(data, symbology: symbology, layoutConfig: layoutConfig)
        #if os(macOS) && arch(arm64)
        return try PaintVmMetalNative.render(scene)
        #elseif os(Windows)
        return try PaintVmDirect2DNative.render(scene)
        #else
        throw Barcode1DError.backendUnavailable
        #endif
    }

    public static func renderPNG(
        _ data: String,
        symbology: String = "code39",
        layoutConfig: Barcode1DLayoutConfig = defaultLayoutConfig
    ) throws -> [UInt8] {
        let pixels = try renderPixels(data, symbology: symbology, layoutConfig: layoutConfig)
        return try PaintCodecPNGNative.encode(pixels)
    }
}
