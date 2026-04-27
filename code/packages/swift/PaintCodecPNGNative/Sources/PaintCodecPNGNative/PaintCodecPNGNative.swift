#if canImport(CPaintCodecPNGNative)
import CPaintCodecPNGNative
#endif
import PixelContainer

public enum PaintCodecPNGNativeError: Error, Equatable {
    case encodeFailed
}

public enum PaintCodecPNGNative {
    public static func encode(_ pixels: PixelContainer) throws -> [UInt8] {
        guard pixels.data.count == Int(pixels.width) * Int(pixels.height) * 4 else {
            throw PaintCodecPNGNativeError.encodeFailed
        }

        #if canImport(CPaintCodecPNGNative)
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
        #else
        return encodePortablePNG(pixels)
        #endif
    }
}

private func encodePortablePNG(_ pixels: PixelContainer) -> [UInt8] {
    let width = Int(pixels.width)
    let height = Int(pixels.height)
    let rowStride = width * 4

    var imageData: [UInt8] = []
    imageData.reserveCapacity(height * (rowStride + 1))
    for row in 0..<height {
        imageData.append(0)
        let start = row * rowStride
        imageData.append(contentsOf: pixels.data[start..<(start + rowStride)])
    }

    let compressed = zlibStoredBlocks(imageData)
    var png: [UInt8] = [137, 80, 78, 71, 13, 10, 26, 10]
    appendChunk(
        named: [73, 72, 68, 82],
        data: bigEndianBytes(pixels.width)
            + bigEndianBytes(pixels.height)
            + [8, 6, 0, 0, 0],
        to: &png
    )
    appendChunk(named: [73, 68, 65, 84], data: compressed, to: &png)
    appendChunk(named: [73, 69, 78, 68], data: [], to: &png)
    return png
}

private func zlibStoredBlocks(_ bytes: [UInt8]) -> [UInt8] {
    var output: [UInt8] = [0x78, 0x01]
    var index = 0

    while index < bytes.count {
        let remaining = bytes.count - index
        let blockLength = min(remaining, 65_535)
        let isFinalBlock = index + blockLength == bytes.count
        let len = UInt16(blockLength)
        let nlen = ~len

        output.append(isFinalBlock ? 0x01 : 0x00)
        output.append(UInt8(len & 0x00ff))
        output.append(UInt8((len >> 8) & 0x00ff))
        output.append(UInt8(nlen & 0x00ff))
        output.append(UInt8((nlen >> 8) & 0x00ff))
        output.append(contentsOf: bytes[index..<(index + blockLength)])

        index += blockLength
    }

    output.append(contentsOf: bigEndianBytes(adler32(bytes)))
    return output
}

private func appendChunk(named name: [UInt8], data: [UInt8], to output: inout [UInt8]) {
    output.append(contentsOf: bigEndianBytes(UInt32(data.count)))
    output.append(contentsOf: name)
    output.append(contentsOf: data)
    output.append(contentsOf: bigEndianBytes(crc32(name + data)))
}

private func bigEndianBytes(_ value: UInt32) -> [UInt8] {
    [
        UInt8((value >> 24) & 0xff),
        UInt8((value >> 16) & 0xff),
        UInt8((value >> 8) & 0xff),
        UInt8(value & 0xff),
    ]
}

private func adler32(_ bytes: [UInt8]) -> UInt32 {
    let modulus: UInt32 = 65_521
    var a: UInt32 = 1
    var b: UInt32 = 0

    for byte in bytes {
        a = (a + UInt32(byte)) % modulus
        b = (b + a) % modulus
    }

    return (b << 16) | a
}

private func crc32(_ bytes: [UInt8]) -> UInt32 {
    var crc: UInt32 = 0xffff_ffff

    for byte in bytes {
        crc ^= UInt32(byte)
        for _ in 0..<8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >>= 1
            }
        }
    }

    return crc ^ 0xffff_ffff
}
