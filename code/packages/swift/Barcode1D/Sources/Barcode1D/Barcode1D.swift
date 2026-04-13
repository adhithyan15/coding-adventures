import Foundation
import BarcodeLayout1D
import Code39
import PaintCodecPNGNative
import PaintInstructions
import PaintVmMetalNative
import PixelContainer

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
        #else
        return nil
        #endif
    }

    private static func normalizeSymbology(_ symbology: String) throws -> String {
        let normalized = symbology.replacingOccurrences(of: "-", with: "")
            .replacingOccurrences(of: "_", with: "")
            .lowercased()
        if normalized == "code39" {
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
        case "code39":
            return try layoutCode39(data, config: layoutConfig)
        default:
            throw Barcode1DError.unsupportedSymbology(symbology)
        }
    }

    public static func renderPixels(
        _ data: String,
        symbology: String = "code39",
        layoutConfig: Barcode1DLayoutConfig = defaultLayoutConfig
    ) throws -> PixelContainer {
        guard currentBackend() == "metal" else {
            throw Barcode1DError.backendUnavailable
        }
        let scene = try buildScene(data, symbology: symbology, layoutConfig: layoutConfig)
        return try PaintVmMetalNative.render(scene)
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
