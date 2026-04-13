import CPaintCodecPNGNative
import PixelContainer

public enum PaintCodecPNGNativeError: Error, Equatable {
    case encodeFailed
}

public enum PaintCodecPNGNative {
    public static func encode(_ pixels: PixelContainer) throws -> [UInt8] {
        var outBytes = paint_encoded_bytes_t(data: nil, len: 0)
        let result = pixels.data.withUnsafeBufferPointer { dataBuffer in
            paint_codec_png_encode_rgba8(
                pixels.width,
                pixels.height,
                dataBuffer.baseAddress,
                dataBuffer.count,
                &outBytes
            )
        }

        guard result == 1, let dataPointer = outBytes.data else {
            throw PaintCodecPNGNativeError.encodeFailed
        }

        defer {
            paint_codec_png_free_bytes(dataPointer, outBytes.len)
        }

        return Array(UnsafeBufferPointer(start: dataPointer, count: Int(outBytes.len)))
    }
}
