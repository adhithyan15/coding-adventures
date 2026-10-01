import Foundation
import PaintInstructions
import SHA256
import XCTest
@testable import BarcodeLayout1D

private enum FixtureError: Error {
    case invalid(String)
}

/// A deliberately small structural scanner runs before Foundation decoding.
/// It bounds nesting and observes every object key, which JSONSerialization
/// alone cannot do because it silently keeps only the last duplicate.
private struct JSONEnvelopeScanner {
    let bytes: [UInt8]
    let depthLimit: Int
    private(set) var index = 0

    mutating func scan() throws {
        try whitespace()
        try value(depth: 0)
        try whitespace()
        guard index == bytes.count else { throw FixtureError.invalid("fixture-invalid-json") }
    }

    private mutating func value(depth: Int) throws {
        guard index < bytes.count else { throw FixtureError.invalid("fixture-invalid-json") }
        switch bytes[index] {
        case 0x7B: try object(depth: depth + 1)
        case 0x5B: try array(depth: depth + 1)
        case 0x22: _ = try string()
        case 0x74: try literal("true")
        case 0x66: try literal("false")
        case 0x6E: try literal("null")
        default: try number()
        }
    }

    private mutating func object(depth: Int) throws {
        guard depth <= depthLimit else { throw FixtureError.invalid("fixture-depth-limit") }
        index += 1
        try whitespace()
        if consume(0x7D) { return }
        var keys = Set<String>()
        while true {
            let key = try string()
            guard keys.insert(key).inserted else { throw FixtureError.invalid("fixture-invalid-json") }
            try whitespace()
            guard consume(0x3A) else { throw FixtureError.invalid("fixture-invalid-json") }
            try whitespace()
            try value(depth: depth)
            try whitespace()
            if consume(0x7D) { return }
            guard consume(0x2C) else { throw FixtureError.invalid("fixture-invalid-json") }
            try whitespace()
        }
    }

    private mutating func array(depth: Int) throws {
        guard depth <= depthLimit else { throw FixtureError.invalid("fixture-depth-limit") }
        index += 1
        try whitespace()
        if consume(0x5D) { return }
        while true {
            try value(depth: depth)
            try whitespace()
            if consume(0x5D) { return }
            guard consume(0x2C) else { throw FixtureError.invalid("fixture-invalid-json") }
            try whitespace()
        }
    }

    private mutating func string() throws -> String {
        guard consume(0x22) else { throw FixtureError.invalid("fixture-invalid-json") }
        let start = index - 1
        var escaped = false
        while index < bytes.count {
            let byte = bytes[index]
            index += 1
            if escaped {
                escaped = false
            } else if byte == 0x5C {
                escaped = true
            } else if byte == 0x22 {
                let fragment = Data(bytes[start..<index])
                do {
                    return try JSONDecoder().decode(String.self, from: fragment)
                } catch {
                    throw FixtureError.invalid("fixture-invalid-scalar")
                }
            } else if byte < 0x20 {
                throw FixtureError.invalid("fixture-invalid-json")
            }
        }
        throw FixtureError.invalid("fixture-invalid-json")
    }

    private mutating func literal(_ text: String) throws {
        let expected = Array(text.utf8)
        guard index + expected.count <= bytes.count,
              Array(bytes[index..<(index + expected.count)]) == expected
        else { throw FixtureError.invalid("fixture-invalid-json") }
        index += expected.count
    }

    private mutating func number() throws {
        let start = index
        while index < bytes.count, ![0x20, 0x09, 0x0A, 0x0D, 0x2C, 0x5D, 0x7D].contains(bytes[index]) {
            index += 1
        }
        guard index > start else { throw FixtureError.invalid("fixture-invalid-json") }
    }

    private mutating func whitespace() throws {
        while index < bytes.count, [0x20, 0x09, 0x0A, 0x0D].contains(bytes[index]) {
            index += 1
        }
    }

    private mutating func consume(_ byte: UInt8) -> Bool {
        guard index < bytes.count, bytes[index] == byte else { return false }
        index += 1
        return true
    }
}

final class BarcodeLayout1DConformanceTests: XCTestCase {
    private static let maxFixtureBytes = 131_072
    private static let fixtureRoot = URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .deletingLastPathComponent()
        .appendingPathComponent("specs/fixtures/barcode-layout-1d-v1")

    private static func readBounded(_ url: URL) throws -> Data {
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        let data = try handle.read(upToCount: maxFixtureBytes + 1) ?? Data()
        guard data.count <= maxFixtureBytes else { throw FixtureError.invalid("fixture-size-limit") }
        return data
    }

    private static func parseBounded(_ data: Data, depthLimit: Int = 8) throws -> Any {
        guard data.count <= maxFixtureBytes else { throw FixtureError.invalid("fixture-size-limit") }
        guard String(data: data, encoding: .utf8) != nil else { throw FixtureError.invalid("fixture-invalid-json") }
        var scanner = JSONEnvelopeScanner(bytes: Array(data), depthLimit: depthLimit)
        try scanner.scan()
        do {
            return try JSONSerialization.jsonObject(with: data, options: [.fragmentsAllowed])
        } catch {
            throw FixtureError.invalid("fixture-invalid-json")
        }
    }

    private static func loadDocument(schemaData: Data, documentData: Data) throws -> [String: Any] {
        let schema = try parseBounded(schemaData, depthLimit: 24)
        let document = try parseBounded(documentData)
        guard let schemaObject = schema as? [String: Any],
              let object = document as? [String: Any]
        else { throw FixtureError.invalid("fixture-schema-invalid-root") }
        guard try int(object["schema_version"]) == 1,
              object["profile"] as? String == "barcode-layout-1d-v1",
              object["limits"] is [String: Any],
              let errorIDs = object["error_ids"] as? [Any],
              errorIDs.allSatisfy({ $0 is String }),
              let cases = object["cases"] as? [[String: Any]],
              (1...64).contains(cases.count)
        else { throw FixtureError.invalid("fixture-schema-invalid-envelope") }
        try validateLocalReferences(schemaObject)
        var identifiers = Set<String>()
        let operations = Set(["expand-binary", "expand-width", "compute-layout", "project-scene"])
        for item in cases {
            guard Set(item.keys) == Set(["id", "operation", "input", "expected"]),
                  let identifier = item["id"] as? String,
                  let operation = item["operation"] as? String,
                  operations.contains(operation),
                  item["input"] is [String: Any],
                  item["expected"] is [String: Any],
                  identifiers.insert(identifier).inserted
            else { throw FixtureError.invalid("fixture-schema-invalid") }
        }
        return object
    }

    private static func validateLocalReferences(_ root: Any) throws {
        var stack = [root]
        while let value = stack.popLast() {
            if let array = value as? [Any] {
                stack.append(contentsOf: array)
            } else if let object = value as? [String: Any] {
                for (key, item) in object {
                    if key == "$ref" || key == "$dynamicRef" {
                        guard let reference = item as? String, reference.hasPrefix("#/") else {
                            throw FixtureError.invalid("fixture-schema-invalid")
                        }
                    }
                    stack.append(item)
                }
            }
        }
    }

    private static func int(_ value: Any?) throws -> Int {
        guard let number = value as? NSNumber,
              String(cString: number.objCType) != "c"
        else { throw FixtureError.invalid("fixture-schema-invalid") }
        return number.intValue
    }

    private static func pattern(_ input: [String: Any]) throws -> String {
        if let value = input["pattern"] as? String { return value }
        guard let repeatValue = input["repeat"] as? [String: Any],
              let token = repeatValue["token"] as? String
        else { throw FixtureError.invalid("fixture-schema-invalid") }
        let suffix = repeatValue["suffix"] as? String ?? ""
        let count = try int(repeatValue["count"])
        guard (0...65_569).contains(count), (1...2).contains(token.unicodeScalars.count),
              suffix.unicodeScalars.count <= 1
        else { throw FixtureError.invalid("fixture-schema-invalid") }
        return String(repeating: token, count: count) + suffix
    }

    private static func runs(_ input: [String: Any]) throws -> [Barcode1DRunV1] {
        let rows: [[String: Any]]
        if let explicit = input["runs"] as? [[String: Any]] {
            rows = explicit
        } else {
            guard let repeated = input["repeatRuns"] as? [String: Any],
                  let first = repeated["firstColor"] as? String,
                  let label = repeated["sourceLabel"] as? String,
                  let role = repeated["role"] as? String
            else { throw FixtureError.invalid("fixture-schema-invalid") }
            let count = try int(repeated["count"])
            let modules = try int(repeated["modules"])
            let sourceIndex = try int(repeated["sourceIndex"])
            guard (0...40_980).contains(count) else { throw FixtureError.invalid("fixture-schema-invalid") }
            rows = (0..<count).map { index in
                [
                    "color": index.isMultiple(of: 2) ? first : (first == "bar" ? "space" : "bar"),
                    "modules": modules,
                    "sourceLabel": label,
                    "sourceIndex": sourceIndex,
                    "role": role,
                ]
            }
        }
        return try rows.map { row in
            guard let color = row["color"] as? String,
                  let label = row["sourceLabel"] as? String,
                  let role = row["role"] as? String
            else { throw FixtureError.invalid("fixture-schema-invalid") }
            return Barcode1DRunV1(
                color: color,
                modules: try int(row["modules"]),
                sourceLabel: label,
                sourceIndex: try int(row["sourceIndex"]),
                role: role
            )
        }
    }

    private static func symbols(_ input: [String: Any]) throws -> [BarcodeSymbolDescriptorV1]? {
        if let rows = input["symbols"] as? [[String: Any]] {
            return try rows.map { row in
                guard let label = row["label"] as? String, let role = row["role"] as? String else {
                    throw FixtureError.invalid("fixture-schema-invalid")
                }
                return BarcodeSymbolDescriptorV1(
                    label: label,
                    modules: try int(row["modules"]),
                    sourceIndex: try int(row["sourceIndex"]),
                    role: role
                )
            }
        }
        guard let repeated = input["repeatSymbols"] as? [String: Any] else { return nil }
        guard let label = repeated["label"] as? String, let role = repeated["role"] as? String else {
            throw FixtureError.invalid("fixture-schema-invalid")
        }
        let count = try int(repeated["count"])
        let modules = try int(repeated["modules"])
        guard (0...40_980).contains(count) else { throw FixtureError.invalid("fixture-schema-invalid") }
        return (0..<count).map { BarcodeSymbolDescriptorV1(label: label, modules: modules, sourceIndex: $0, role: role) }
    }

    private static func runObject(_ run: Barcode1DRunV1) -> [String: Any] {
        [
            "color": run.color,
            "modules": run.modules,
            "sourceLabel": run.sourceLabel,
            "sourceIndex": run.sourceIndex,
            "role": run.role,
        ]
    }

    private static func layoutObject(_ layout: Barcode1DLayoutV1) -> [String: Any] {
        [
            "leftQuietZoneModules": layout.leftQuietZoneModules,
            "rightQuietZoneModules": layout.rightQuietZoneModules,
            "contentModules": layout.contentModules,
            "totalModules": layout.totalModules,
            "symbolLayouts": layout.symbolLayouts.map { symbol in
                [
                    "label": symbol.label,
                    "startModule": symbol.startModule,
                    "endModule": symbol.endModule,
                    "sourceIndex": symbol.sourceIndex,
                    "role": symbol.role,
                ]
            },
        ]
    }

    private static func sceneObject(_ scene: PaintScene) -> [String: Any] {
        let rectangles = scene.instructions.compactMap { instruction -> [String: Any]? in
            guard case .rect(let rect) = instruction else { return nil }
            return [
                "x": rect.x,
                "y": rect.y,
                "width": rect.width,
                "height": rect.height,
                "fill": rect.fill,
                "metadata": rect.metadata,
            ]
        }
        return [
            "width": scene.width,
            "height": scene.height,
            "background": scene.background,
            "rectangles": rectangles,
            "metadata": scene.metadata,
        ]
    }

    private static func canonicalData(_ value: Any) throws -> Data {
        try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys, .withoutEscapingSlashes])
    }

    private static func execute(_ testCase: [String: Any]) throws -> Any {
        guard let operation = testCase["operation"] as? String,
              let input = testCase["input"] as? [String: Any]
        else { throw FixtureError.invalid("fixture-schema-invalid") }
        switch operation {
        case "expand-binary":
            return try expandBinaryV1(
                pattern(input),
                sourceLabel: input["sourceLabel"] as! String,
                sourceIndex: int(input["sourceIndex"]),
                role: input["role"] as! String
            )
        case "expand-width":
            return try expandWidthV1(
                pattern(input),
                sourceLabel: input["sourceLabel"] as! String,
                sourceIndex: int(input["sourceIndex"]),
                role: input["role"] as! String,
                narrowMarker: input["narrowMarker"] as? String ?? "N",
                wideMarker: input["wideMarker"] as? String ?? "W",
                narrowModules: try input["narrowModules"].map(int) ?? 1,
                wideModules: try input["wideModules"].map(int) ?? 3,
                startingColor: input["startingColor"] as? String ?? "bar"
            )
        case "compute-layout":
            return try computeLayoutV1(
                runs(input),
                quietZoneModules: int(input["quietZoneModules"]),
                symbols: symbols(input)
            )
        case "project-scene":
            let render = input["renderConfig"] as? [String: Any] ?? [:]
            return try projectSceneV1(
                runs(input),
                quietZoneModules: int(input["quietZoneModules"]),
                options: Barcode1DOptionsV1(
                    renderConfig: Barcode1DRenderConfigV1(
                        moduleWidth: try render["moduleWidth"].map(int) ?? 4,
                        barHeight: try render["barHeight"].map(int) ?? 120,
                        foreground: render["foreground"] as? String ?? "#000000",
                        background: render["background"] as? String ?? "#ffffff",
                        includeHumanReadableText: render["includeHumanReadableText"] as? Bool ?? false
                    ),
                    label: input["label"] as? String ?? "1D barcode",
                    metadata: input["metadata"] as? [String: String] ?? [:],
                    humanReadableText: input["humanReadableText"] as? String,
                    symbols: try symbols(input)
                )
            )
        default:
            throw FixtureError.invalid("fixture-schema-invalid")
        }
    }

    func testLanguageNeutralCorpus() throws {
        let schemaData = try Self.readBounded(Self.fixtureRoot.appendingPathComponent("schema.json"))
        let casesData = try Self.readBounded(Self.fixtureRoot.appendingPathComponent("cases.json"))
        let document = try Self.loadDocument(schemaData: schemaData, documentData: casesData)
        let cases = document["cases"] as! [[String: Any]]
        XCTAssertEqual(cases.count, 56)

        var counts: [String: Int] = [:]
        for testCase in cases {
            let identifier = testCase["id"] as! String
            let operation = testCase["operation"] as! String
            counts[operation, default: 0] += 1
            let expected = testCase["expected"] as! [String: Any]
            do {
                let actual = try Self.execute(testCase)
                if let rows = expected["runs"] as? [[String: Any]] {
                    XCTAssertEqual(try Self.canonicalData((actual as! [Barcode1DRunV1]).map(Self.runObject)), try Self.canonicalData(rows), identifier)
                } else if let digest = expected["runDigest"] as? [String: Any] {
                    let projected = (actual as! [Barcode1DRunV1]).map(Self.runObject)
                    XCTAssertEqual(projected.count, try Self.int(digest["runCount"]), identifier)
                    XCTAssertEqual(projected.reduce(0) { $0 + ($1["modules"] as! Int) }, try Self.int(digest["contentModules"]), identifier)
                    XCTAssertEqual(try Self.canonicalData(projected.first!), try Self.canonicalData(digest["firstRun"]!), identifier)
                    XCTAssertEqual(try Self.canonicalData(projected.last!), try Self.canonicalData(digest["lastRun"]!), identifier)
                    XCTAssertEqual(sha256Hex(try Self.canonicalData(projected)), digest["runsSha256"] as? String, identifier)
                } else if let layout = expected["layout"] {
                    XCTAssertEqual(try Self.canonicalData(Self.layoutObject(actual as! Barcode1DLayoutV1)), try Self.canonicalData(layout), identifier)
                } else {
                    XCTAssertEqual(try Self.canonicalData(Self.sceneObject(actual as! PaintScene)), try Self.canonicalData(expected["scene"]!), identifier)
                }
            } catch let error as BarcodeLayoutV1Error {
                XCTAssertEqual(error.errorID, expected["error"] as? String, identifier)
            }
        }
        XCTAssertEqual(counts, ["expand-binary": 12, "expand-width": 12, "compute-layout": 19, "project-scene": 13])
    }

    func testBoundedLoaderRejectsHostileDocuments() throws {
        let schema = try Self.readBounded(Self.fixtureRoot.appendingPathComponent("schema.json"))
        let cases = try Self.readBounded(Self.fixtureRoot.appendingPathComponent("cases.json"))
        let hostile: [Data] = [
            Data(#"{"duplicate":1,"duplicate":2}"#.utf8),
            Data((String(repeating: "[", count: 9) + "0" + String(repeating: "]", count: 9)).utf8),
            Data([0xFF]),
            Data(String(decoding: cases, as: UTF8.self).replacingOccurrences(of: #""layout-v1-binary-basic""#, with: #""\ud800""#, options: [], range: nil).utf8),
            Data("[]".utf8),
            Data("{}".utf8),
        ]
        for value in hostile {
            XCTAssertThrowsError(try Self.loadDocument(schemaData: schema, documentData: value))
        }
    }

    func testLoaderMaximumByteBoundary() throws {
        let prefix = Data(#"{"padding":""#.utf8)
        let suffix = Data(#""}"#.utf8)
        var exact = prefix
        exact.append(Data(repeating: 0x78, count: Self.maxFixtureBytes - prefix.count - suffix.count))
        exact.append(suffix)
        XCTAssertNoThrow(try Self.parseBounded(exact))
        exact.append(0x20)
        XCTAssertThrowsError(try Self.parseBounded(exact))
    }

    func testTextValueFailsBeforeNativeResolution() {
        var calls = 0
        XCTAssertThrowsError(
            try projectSceneV1(
                [Barcode1DRunV1(color: "bar", modules: 0, sourceLabel: "A", sourceIndex: 0, role: "data")],
                quietZoneModules: 0,
                options: Barcode1DOptionsV1(humanReadableText: "123"),
                nativeResolver: { calls += 1 }
            )
        ) { error in
            XCTAssertEqual((error as? BarcodeLayoutV1Error)?.errorID, "human-readable-text-unsupported")
        }
        XCTAssertEqual(calls, 0)
    }

    func testTextEnabledFailsBeforeNativeResolution() {
        var calls = 0
        XCTAssertThrowsError(
            try projectSceneV1(
                [Barcode1DRunV1(color: "bar", modules: 0, sourceLabel: "A", sourceIndex: 0, role: "data")],
                quietZoneModules: 0,
                options: Barcode1DOptionsV1(renderConfig: Barcode1DRenderConfigV1(moduleWidth: 0, includeHumanReadableText: true)),
                nativeResolver: { calls += 1 }
            )
        ) { error in
            XCTAssertEqual((error as? BarcodeLayoutV1Error)?.errorID, "human-readable-text-unsupported")
        }
        XCTAssertEqual(calls, 0)
    }

    func testOutputsAreIndependentValues() throws {
        let input = [Barcode1DRunV1(color: "bar", modules: 1, sourceLabel: "A", sourceIndex: 0, role: "data")]
        let first = try projectSceneV1(input, quietZoneModules: 1, options: Barcode1DOptionsV1(metadata: ["caller": "value"]))
        var changedMetadata = first.metadata
        changedMetadata["caller"] = "changed"
        var changedInput = input
        changedInput[0] = Barcode1DRunV1(color: "bar", modules: 1, sourceLabel: "changed", sourceIndex: 0, role: "data")
        let second = try projectSceneV1(input, quietZoneModules: 1, options: Barcode1DOptionsV1(metadata: ["caller": "value"]))
        XCTAssertEqual(second.metadata["caller"], "value")
        guard case .rect(let rectangle) = second.instructions[0] else { return XCTFail("expected rectangle") }
        XCTAssertEqual(rectangle.metadata["sourceLabel"], "A")
        XCTAssertEqual(changedMetadata["caller"], "changed")
        XCTAssertEqual(changedInput[0].sourceLabel, "changed")
    }
}
