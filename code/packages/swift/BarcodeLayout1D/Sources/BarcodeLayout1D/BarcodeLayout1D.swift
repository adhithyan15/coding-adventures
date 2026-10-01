import PaintInstructions

public struct Barcode1DRun: Equatable, Sendable {
    public let color: String
    public let modules: Int
    public let sourceCharacter: String
    public let sourceIndex: Int
    public let role: String
    public let metadata: PaintMetadata

    public init(
        color: String,
        modules: Int,
        sourceCharacter: String,
        sourceIndex: Int,
        role: String = "data",
        metadata: PaintMetadata = [:]
    ) {
        self.color = color
        self.modules = modules
        self.sourceCharacter = sourceCharacter
        self.sourceIndex = sourceIndex
        self.role = role
        self.metadata = metadata
    }
}

public struct Barcode1DLayoutConfig: Equatable, Sendable {
    public let moduleUnit: Int
    public let barHeight: Int
    public let quietZoneModules: Int

    public init(moduleUnit: Int = 4, barHeight: Int = 120, quietZoneModules: Int = 10) {
        self.moduleUnit = moduleUnit
        self.barHeight = barHeight
        self.quietZoneModules = quietZoneModules
    }
}

public struct PaintBarcode1DOptions: Equatable, Sendable {
    public let fill: String
    public let background: String
    public let metadata: PaintMetadata

    public init(fill: String = "#000000", background: String = "#ffffff", metadata: PaintMetadata = [:]) {
        self.fill = fill
        self.background = background
        self.metadata = metadata
    }
}

public let defaultBarcode1DLayoutConfig = Barcode1DLayoutConfig()
public let defaultPaintBarcode1DOptions = PaintBarcode1DOptions()

public enum BarcodeLayout1DError: Error, Equatable {
    case invalidConfiguration(String)
}

private func validate(_ config: Barcode1DLayoutConfig) throws {
    if config.moduleUnit <= 0 {
        throw BarcodeLayout1DError.invalidConfiguration("moduleUnit must be positive")
    }
    if config.barHeight <= 0 {
        throw BarcodeLayout1DError.invalidConfiguration("barHeight must be positive")
    }
    if config.quietZoneModules < 0 {
        throw BarcodeLayout1DError.invalidConfiguration("quietZoneModules must be non-negative")
    }
}

private func validate(_ run: Barcode1DRun) throws {
    if run.color != "bar" && run.color != "space" {
        throw BarcodeLayout1DError.invalidConfiguration("run color must be 'bar' or 'space'")
    }
    if run.modules <= 0 {
        throw BarcodeLayout1DError.invalidConfiguration("run modules must be positive")
    }
}

public func runsFromBinaryPattern(
    _ pattern: String,
    barCharacter: Character = "1",
    spaceCharacter: Character = "0",
    sourceCharacter: String = "",
    sourceIndex: Int = 0,
    metadata: PaintMetadata = [:]
) throws -> [Barcode1DRun] {
    guard let first = pattern.first else {
        return []
    }

    var runs: [Barcode1DRun] = []
    var current = first
    var count = 1

    func flush(_ token: Character, modules: Int) throws {
        let color: String
        if token == barCharacter {
            color = "bar"
        } else if token == spaceCharacter {
            color = "space"
        } else {
            throw BarcodeLayout1DError.invalidConfiguration("binary pattern contains unsupported token")
        }

        runs.append(
            Barcode1DRun(
                color: color,
                modules: modules,
                sourceCharacter: sourceCharacter,
                sourceIndex: sourceIndex,
                metadata: metadata
            )
        )
    }

    for token in pattern.dropFirst() {
        if token == current {
            count += 1
            continue
        }
        try flush(current, modules: count)
        current = token
        count = 1
    }

    try flush(current, modules: count)
    return runs
}

public func runsFromWidthPattern(
    _ pattern: String,
    colors: [String],
    sourceCharacter: String,
    sourceIndex: Int,
    narrowModules: Int = 1,
    wideModules: Int = 3,
    role: String = "data",
    metadata: PaintMetadata = [:]
) throws -> [Barcode1DRun] {
    if pattern.count != colors.count {
        throw BarcodeLayout1DError.invalidConfiguration("pattern length must match colors length")
    }
    if narrowModules <= 0 || wideModules <= 0 {
        throw BarcodeLayout1DError.invalidConfiguration("module widths must be positive")
    }

    return try zip(pattern, colors).map { element, color in
        let modules: Int
        switch element {
        case "N":
            modules = narrowModules
        case "W":
            modules = wideModules
        default:
            throw BarcodeLayout1DError.invalidConfiguration("width pattern contains unsupported token")
        }

        return Barcode1DRun(
            color: color,
            modules: modules,
            sourceCharacter: sourceCharacter,
            sourceIndex: sourceIndex,
            role: role,
            metadata: metadata
        )
    }
}

public func layoutBarcode1D(
    _ runs: [Barcode1DRun],
    config: Barcode1DLayoutConfig = defaultBarcode1DLayoutConfig,
    options: PaintBarcode1DOptions = defaultPaintBarcode1DOptions
) throws -> PaintScene {
    try validate(config)

    let quietZoneWidth = config.quietZoneModules * config.moduleUnit
    var cursorX = quietZoneWidth
    var instructions: [PaintInstruction] = []

    for run in runs {
        try validate(run)
        let width = run.modules * config.moduleUnit
        if run.color == "bar" {
            instructions.append(
                paintRect(
                    x: cursorX,
                    y: 0,
                    width: width,
                    height: config.barHeight,
                    fill: options.fill,
                    metadata: [
                        "source_char": run.sourceCharacter,
                        "source_index": String(run.sourceIndex),
                        "modules": String(run.modules),
                        "role": run.role,
                    ].merging(run.metadata) { _, rhs in rhs }
                )
            )
        }
        cursorX += width
    }

    let contentWidth = cursorX - quietZoneWidth
    return paintScene(
        width: cursorX + quietZoneWidth,
        height: config.barHeight,
        instructions: instructions,
        background: options.background,
        metadata: [
            "content_width": String(contentWidth),
            "quiet_zone_width": String(quietZoneWidth),
            "module_unit": String(config.moduleUnit),
            "bar_height": String(config.barHeight),
        ].merging(options.metadata) { _, rhs in rhs }
    )
}

public func drawOneDimensionalBarcode(
    _ runs: [Barcode1DRun],
    config: Barcode1DLayoutConfig = defaultBarcode1DLayoutConfig,
    options: PaintBarcode1DOptions = defaultPaintBarcode1DOptions
) throws -> PaintScene {
    try layoutBarcode1D(runs, config: config, options: options)
}

// MARK: - Language-neutral barcode-layout-1d-v1 adapter

public struct Barcode1DRunV1: Equatable, Sendable {
    public let color: String
    public let modules: Int
    public let sourceLabel: String
    public let sourceIndex: Int
    public let role: String

    public init(color: String, modules: Int, sourceLabel: String, sourceIndex: Int, role: String) {
        self.color = color
        self.modules = modules
        self.sourceLabel = sourceLabel
        self.sourceIndex = sourceIndex
        self.role = role
    }
}

public struct BarcodeSymbolDescriptorV1: Equatable, Sendable {
    public let label: String
    public let modules: Int
    public let sourceIndex: Int
    public let role: String

    public init(label: String, modules: Int, sourceIndex: Int, role: String) {
        self.label = label
        self.modules = modules
        self.sourceIndex = sourceIndex
        self.role = role
    }
}

public struct BarcodeSymbolLayoutV1: Equatable, Sendable {
    public let label: String
    public let startModule: Int
    public let endModule: Int
    public let sourceIndex: Int
    public let role: String
}

public struct Barcode1DLayoutV1: Equatable, Sendable {
    public let leftQuietZoneModules: Int
    public let rightQuietZoneModules: Int
    public let contentModules: Int
    public let totalModules: Int
    public let symbolLayouts: [BarcodeSymbolLayoutV1]
}

public struct Barcode1DRenderConfigV1: Equatable, Sendable {
    public let moduleWidth: Int
    public let barHeight: Int
    public let foreground: String
    public let background: String
    public let includeHumanReadableText: Bool

    public init(
        moduleWidth: Int = 4,
        barHeight: Int = 120,
        foreground: String = "#000000",
        background: String = "#ffffff",
        includeHumanReadableText: Bool = false
    ) {
        self.moduleWidth = moduleWidth
        self.barHeight = barHeight
        self.foreground = foreground
        self.background = background
        self.includeHumanReadableText = includeHumanReadableText
    }
}

public struct Barcode1DOptionsV1: Equatable, Sendable {
    public let renderConfig: Barcode1DRenderConfigV1
    public let label: String
    public let metadata: [String: String]
    public let humanReadableText: String?
    public let symbols: [BarcodeSymbolDescriptorV1]?

    public init(
        renderConfig: Barcode1DRenderConfigV1 = Barcode1DRenderConfigV1(),
        label: String = "1D barcode",
        metadata: [String: String] = [:],
        humanReadableText: String? = nil,
        symbols: [BarcodeSymbolDescriptorV1]? = nil
    ) {
        self.renderConfig = renderConfig
        self.label = label
        self.metadata = metadata
        self.humanReadableText = humanReadableText
        self.symbols = symbols
    }
}

public struct BarcodeLayoutV1Error: Error, Equatable, Sendable {
    public let errorID: String

    public init(_ errorID: String) {
        self.errorID = errorID
    }
}

private enum BarcodeV1Limit {
    static let patternScalars = 65_567
    static let runs = 40_979
    static let contentModules = 65_567
    static let quietZoneModules = 4_096
    static let symbols = 40_979
    static let labelScalars = 4_096
    static let metadataEntries = 64
    static let metadataKeyScalars = 128
    static let metadataValueScalars = 4_096
    static let metadataBytes = 65_536
    static let renderDimension = 8_192
    static let colorScalars = 128
}

private let barcodeV1Roles = Set(["data", "start", "stop", "guard", "check", "inter-character-gap"])
private let barcodeV1SymbolRoles = Set(["data", "start", "stop", "guard", "check"])

@inline(__always)
private func barcodeV1Fail(_ errorID: String) throws -> Never {
    throw BarcodeLayoutV1Error(errorID)
}

private func barcodeV1ValidateSource(_ label: String, _ sourceIndex: Int) throws {
    guard label.unicodeScalars.count <= BarcodeV1Limit.labelScalars,
          sourceIndex >= Int(Int32.min), sourceIndex <= Int(Int32.max)
    else { try barcodeV1Fail("invalid-source-attribution") }
}

private func barcodeV1ValidateRuns(_ runs: [Barcode1DRunV1]) throws -> Int {
    guard runs.count <= BarcodeV1Limit.runs else { try barcodeV1Fail("too-many-runs") }
    var content = 0
    var previousColor: String?
    for run in runs {
        guard run.color == "bar" || run.color == "space", barcodeV1Roles.contains(run.role) else {
            try barcodeV1Fail("invalid-source-attribution")
        }
        try barcodeV1ValidateSource(run.sourceLabel, run.sourceIndex)
        guard run.modules > 0 else { try barcodeV1Fail("invalid-module-count") }
        guard previousColor != run.color else { try barcodeV1Fail("non-alternating-runs") }
        guard run.modules <= BarcodeV1Limit.contentModules - content else {
            try barcodeV1Fail("content-too-wide")
        }
        content += run.modules
        previousColor = run.color
    }
    return content
}

public func expandBinaryV1(
    _ pattern: String,
    sourceLabel: String,
    sourceIndex: Int,
    role: String
) throws -> [Barcode1DRunV1] {
    let scalarCount = pattern.unicodeScalars.prefix(BarcodeV1Limit.patternScalars + 1).count
    guard scalarCount <= BarcodeV1Limit.patternScalars else { try barcodeV1Fail("pattern-too-long") }
    let tokens = Array(pattern.unicodeScalars)
    guard !tokens.isEmpty else { try barcodeV1Fail("empty-pattern") }
    guard tokens.allSatisfy({ $0.value == 48 || $0.value == 49 }) else {
        try barcodeV1Fail("invalid-binary-token")
    }
    try barcodeV1ValidateSource(sourceLabel, sourceIndex)
    guard barcodeV1Roles.contains(role) else { try barcodeV1Fail("invalid-source-attribution") }

    var result: [Barcode1DRunV1] = []
    result.reserveCapacity(min(tokens.count, BarcodeV1Limit.runs))
    var current = tokens[0]
    var width = 1
    for token in tokens.dropFirst() {
        if token == current {
            width += 1
        } else {
            guard result.count < BarcodeV1Limit.runs else { try barcodeV1Fail("too-many-runs") }
            result.append(Barcode1DRunV1(
                color: current.value == 49 ? "bar" : "space",
                modules: width,
                sourceLabel: sourceLabel,
                sourceIndex: sourceIndex,
                role: role
            ))
            current = token
            width = 1
        }
    }
    guard result.count < BarcodeV1Limit.runs else { try barcodeV1Fail("too-many-runs") }
    result.append(Barcode1DRunV1(
        color: current.value == 49 ? "bar" : "space",
        modules: width,
        sourceLabel: sourceLabel,
        sourceIndex: sourceIndex,
        role: role
    ))
    return result
}

public func expandWidthV1(
    _ pattern: String,
    sourceLabel: String,
    sourceIndex: Int,
    role: String,
    narrowMarker: String = "N",
    wideMarker: String = "W",
    narrowModules: Int = 1,
    wideModules: Int = 3,
    startingColor: String = "bar"
) throws -> [Barcode1DRunV1] {
    let scalarCount = pattern.unicodeScalars.prefix(BarcodeV1Limit.patternScalars + 1).count
    guard scalarCount <= BarcodeV1Limit.patternScalars else { try barcodeV1Fail("pattern-too-long") }
    let tokens = Array(pattern.unicodeScalars)
    guard !tokens.isEmpty else { try barcodeV1Fail("empty-pattern") }
    guard narrowMarker.unicodeScalars.count == 1,
          wideMarker.unicodeScalars.count == 1,
          narrowMarker != wideMarker
    else { try barcodeV1Fail("invalid-marker-configuration") }
    let narrow = narrowMarker.unicodeScalars.first!
    let wide = wideMarker.unicodeScalars.first!
    guard tokens.allSatisfy({ $0 == narrow || $0 == wide }) else { try barcodeV1Fail("invalid-width-token") }
    guard narrowModules > 0, wideModules > 0 else { try barcodeV1Fail("invalid-module-count") }
    try barcodeV1ValidateSource(sourceLabel, sourceIndex)
    guard barcodeV1Roles.contains(role) else { try barcodeV1Fail("invalid-source-attribution") }
    guard startingColor == "bar" || startingColor == "space" else {
        try barcodeV1Fail("invalid-marker-configuration")
    }
    guard tokens.count <= BarcodeV1Limit.runs else { try barcodeV1Fail("too-many-runs") }

    var result: [Barcode1DRunV1] = []
    result.reserveCapacity(tokens.count)
    var content = 0
    for (index, token) in tokens.enumerated() {
        let modules = token == wide ? wideModules : narrowModules
        guard modules <= BarcodeV1Limit.contentModules - content else { try barcodeV1Fail("content-too-wide") }
        content += modules
        let color = index.isMultiple(of: 2) ? startingColor : (startingColor == "bar" ? "space" : "bar")
        result.append(Barcode1DRunV1(
            color: color,
            modules: modules,
            sourceLabel: sourceLabel,
            sourceIndex: sourceIndex,
            role: role
        ))
    }
    return result
}

public func computeLayoutV1(
    _ runs: [Barcode1DRunV1],
    quietZoneModules: Int,
    symbols: [BarcodeSymbolDescriptorV1]? = nil
) throws -> Barcode1DLayoutV1 {
    let content = try barcodeV1ValidateRuns(runs)
    guard (1...BarcodeV1Limit.quietZoneModules).contains(quietZoneModules) else {
        try barcodeV1Fail("invalid-quiet-zone")
    }
    let total = quietZoneModules + content + quietZoneModules
    var layouts: [BarcodeSymbolLayoutV1] = []

    if let symbols {
        guard symbols.count <= BarcodeV1Limit.symbols else { try barcodeV1Fail("too-many-symbols") }
        var cursor = 0
        for symbol in symbols {
            guard symbol.modules > 0 else { try barcodeV1Fail("invalid-module-count") }
            try barcodeV1ValidateSource(symbol.label, symbol.sourceIndex)
            guard barcodeV1SymbolRoles.contains(symbol.role) else { try barcodeV1Fail("invalid-source-attribution") }
            guard symbol.modules <= BarcodeV1Limit.contentModules - cursor else {
                try barcodeV1Fail("symbol-width-mismatch")
            }
            let end = cursor + symbol.modules
            layouts.append(BarcodeSymbolLayoutV1(
                label: symbol.label,
                startModule: cursor,
                endModule: end,
                sourceIndex: symbol.sourceIndex,
                role: symbol.role
            ))
            cursor = end
        }
        guard cursor == content else { try barcodeV1Fail("symbol-width-mismatch") }
    } else {
        var cursor = 0
        var current: BarcodeSymbolLayoutV1?
        for run in runs {
            if run.role != "inter-character-gap" {
                let same = current.map {
                    $0.label == run.sourceLabel && $0.sourceIndex == run.sourceIndex && $0.role == run.role
                } ?? false
                if !same {
                    if let current { layouts.append(current) }
                    current = BarcodeSymbolLayoutV1(
                        label: run.sourceLabel,
                        startModule: cursor,
                        endModule: cursor,
                        sourceIndex: run.sourceIndex,
                        role: run.role
                    )
                }
            }
            cursor += run.modules
            if let value = current {
                current = BarcodeSymbolLayoutV1(
                    label: value.label,
                    startModule: value.startModule,
                    endModule: cursor,
                    sourceIndex: value.sourceIndex,
                    role: value.role
                )
            }
        }
        if let current { layouts.append(current) }
        guard layouts.count <= BarcodeV1Limit.symbols else { try barcodeV1Fail("too-many-symbols") }
    }

    return Barcode1DLayoutV1(
        leftQuietZoneModules: quietZoneModules,
        rightQuietZoneModules: quietZoneModules,
        contentModules: content,
        totalModules: total,
        symbolLayouts: layouts
    )
}

public func projectSceneV1(
    _ runs: [Barcode1DRunV1],
    quietZoneModules: Int,
    options: Barcode1DOptionsV1 = Barcode1DOptionsV1(),
    nativeResolver: (() -> Void)? = nil
) throws -> PaintScene {
    if options.renderConfig.includeHumanReadableText || options.humanReadableText != nil {
        try barcodeV1Fail("human-readable-text-unsupported")
    }
    let config = options.renderConfig
    guard (1...BarcodeV1Limit.renderDimension).contains(config.moduleWidth),
          (1...BarcodeV1Limit.renderDimension).contains(config.barHeight),
          config.foreground.unicodeScalars.count <= BarcodeV1Limit.colorScalars,
          config.background.unicodeScalars.count <= BarcodeV1Limit.colorScalars
    else { try barcodeV1Fail("invalid-render-config") }

    // The portable entry point deliberately never invokes a native resolver.
    _ = nativeResolver
    let layout = try computeLayoutV1(runs, quietZoneModules: quietZoneModules, symbols: options.symbols)
    guard options.metadata.count <= BarcodeV1Limit.metadataEntries,
          options.label.unicodeScalars.count <= BarcodeV1Limit.labelScalars
    else { try barcodeV1Fail("metadata-too-large") }
    var metadataBytes = 0
    for (key, value) in options.metadata {
        guard key.unicodeScalars.count <= BarcodeV1Limit.metadataKeyScalars,
              value.unicodeScalars.count <= BarcodeV1Limit.metadataValueScalars
        else { try barcodeV1Fail("metadata-too-large") }
        let addition = key.utf8.count + value.utf8.count
        guard addition <= BarcodeV1Limit.metadataBytes - metadataBytes else {
            try barcodeV1Fail("metadata-too-large")
        }
        metadataBytes += addition
    }

    let sceneWidth = layout.totalModules * config.moduleWidth
    var cursor = layout.leftQuietZoneModules
    var instructions: [PaintInstruction] = []
    instructions.reserveCapacity(runs.count)
    for run in runs {
        let end = cursor + run.modules
        if run.color == "bar" {
            instructions.append(paintRect(
                x: cursor * config.moduleWidth,
                y: 0,
                width: run.modules * config.moduleWidth,
                height: config.barHeight,
                fill: config.foreground,
                metadata: [
                    "sourceLabel": run.sourceLabel,
                    "sourceIndex": String(run.sourceIndex),
                    "role": run.role,
                    "moduleStart": String(cursor),
                    "moduleEnd": String(end),
                ]
            ))
        }
        cursor = end
    }

    var metadata = options.metadata
    metadata["label"] = options.label
    metadata["leftQuietZoneModules"] = String(layout.leftQuietZoneModules)
    metadata["rightQuietZoneModules"] = String(layout.rightQuietZoneModules)
    metadata["contentModules"] = String(layout.contentModules)
    metadata["totalModules"] = String(layout.totalModules)
    metadata["moduleWidthPx"] = String(config.moduleWidth)
    metadata["barHeightPx"] = String(config.barHeight)
    metadata["sceneWidthPx"] = String(sceneWidth)
    metadata["sceneHeightPx"] = String(config.barHeight)
    metadata["symbolCount"] = String(layout.symbolLayouts.count)
    return paintScene(
        width: sceneWidth,
        height: config.barHeight,
        instructions: instructions,
        background: config.background,
        metadata: metadata
    )
}
