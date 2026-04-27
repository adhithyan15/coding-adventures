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
