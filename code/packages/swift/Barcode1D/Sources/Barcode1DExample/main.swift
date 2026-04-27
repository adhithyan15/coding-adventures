import Barcode1D
import Foundation

let arguments = CommandLine.arguments
let outputPath = arguments.count > 1 ? arguments[1] : NSTemporaryDirectory() + "/swift-metal-code39.png"
let data = arguments.count > 2 ? arguments[2] : "HELLO-123"

do {
    let png = try Barcode1D.renderPNG(data)
    try Data(png).write(to: URL(fileURLWithPath: outputPath))
    print(outputPath)
} catch {
    fputs("failed to render barcode: \(error)\n", stderr)
    exit(1)
}
