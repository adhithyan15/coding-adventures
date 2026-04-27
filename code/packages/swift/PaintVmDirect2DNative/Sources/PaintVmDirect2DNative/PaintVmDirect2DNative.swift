#if os(Windows)
import CPaintVmDirect2DNative
#endif
import PaintInstructions
import PixelContainer

public enum PaintVmDirect2DNativeError: Error, Equatable {
    case renderFailed
}

#if os(Windows)
private func toCColor(_ color: PaintColorRGBA8) -> paint_rgba8_color_t {
    paint_rgba8_color_t(r: color.r, g: color.g, b: color.b, a: color.a)
}
#endif

public enum PaintVmDirect2DNative {
    public static func render(_ scene: PaintScene) throws -> PixelContainer {
        #if os(Windows)
        let rects = scene.instructions.map { instruction in
            paint_rect_instruction_t(
                x: UInt32(instruction.x),
                y: UInt32(instruction.y),
                width: UInt32(instruction.width),
                height: UInt32(instruction.height),
                fill: toCColor(parsePaintColor(instruction.fill))
            )
        }

        var outBuffer = paint_rgba8_buffer_t(width: 0, height: 0, data: nil, len: 0)
        let result = rects.withUnsafeBufferPointer { rectBuffer in
            paint_vm_direct2d_render_rect_scene(
                UInt32(scene.width),
                UInt32(scene.height),
                toCColor(parsePaintColor(scene.background)),
                rectBuffer.baseAddress,
                rectBuffer.count,
                &outBuffer
            )
        }

        guard result == 1, let dataPointer = outBuffer.data else {
            throw PaintVmDirect2DNativeError.renderFailed
        }

        defer {
            paint_vm_direct2d_free_buffer_data(dataPointer, outBuffer.len)
        }

        let bytes = Array(UnsafeBufferPointer(start: dataPointer, count: Int(outBuffer.len)))
        var pixels = PixelContainer(width: outBuffer.width, height: outBuffer.height)
        pixels.data = bytes
        return pixels
        #else
        _ = scene
        throw PaintVmDirect2DNativeError.renderFailed
        #endif
    }
}
