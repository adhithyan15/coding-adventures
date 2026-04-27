import Foundation

public enum BuildToolError: Error, LocalizedError {
    case invalidArguments(String)
    case cycleDetected
    case unsupportedPlanVersion(Int, Int)
    case invalidPlan(String)
    case io(String)

    public var errorDescription: String? {
        switch self {
        case let .invalidArguments(message):
            return message
        case .cycleDetected:
            return "dependency graph contains a cycle"
        case let .unsupportedPlanVersion(found, supported):
            return "unsupported build plan version \(found) (this tool supports up to \(supported))"
        case let .invalidPlan(message):
            return message
        case let .io(message):
            return message
        }
    }
}

public enum BuildStatus: String, Codable {
    case built
    case failed
    case skipped
    case depSkipped = "dep-skipped"
    case wouldBuild = "would-build"
}

public struct BuildPackage: Codable, Equatable {
    public var name: String
    public var path: String
    public var buildCommands: [String]
    public var language: String
    public var buildContent: String
    public var isStarlark: Bool
    public var declaredSrcs: [String]
    public var declaredDeps: [String]

    public init(
        name: String,
        path: String,
        buildCommands: [String] = [],
        language: String = "unknown",
        buildContent: String = "",
        isStarlark: Bool = false,
        declaredSrcs: [String] = [],
        declaredDeps: [String] = []
    ) {
        self.name = name
        self.path = path
        self.buildCommands = buildCommands
        self.language = language
        self.buildContent = buildContent
        self.isStarlark = isStarlark
        self.declaredSrcs = declaredSrcs
        self.declaredDeps = declaredDeps
    }
}

public struct BuildTarget: Equatable {
    public var rule: String
    public var name: String
    public var srcs: [String]
    public var deps: [String]
    public var testRunner: String
    public var entryPoint: String

    public init(
        rule: String,
        name: String,
        srcs: [String] = [],
        deps: [String] = [],
        testRunner: String = "",
        entryPoint: String = ""
    ) {
        self.rule = rule
        self.name = name
        self.srcs = srcs
        self.deps = deps
        self.testRunner = testRunner
        self.entryPoint = entryPoint
    }
}

public struct BuildFileEvaluation: Equatable {
    public var targets: [BuildTarget]

    public init(targets: [BuildTarget]) {
        self.targets = targets
    }
}

public struct PackageBuildResult: Equatable {
    public var packageName: String
    public var status: BuildStatus
    public var duration: TimeInterval
    public var stdout: String
    public var stderr: String
    public var returnCode: Int32

    public init(
        packageName: String,
        status: BuildStatus,
        duration: TimeInterval = 0,
        stdout: String = "",
        stderr: String = "",
        returnCode: Int32 = 0
    ) {
        self.packageName = packageName
        self.status = status
        self.duration = duration
        self.stdout = stdout
        self.stderr = stderr
        self.returnCode = returnCode
    }
}

public struct CacheEntry: Codable, Equatable {
    public var packageHash: String
    public var depsHash: String
    public var lastBuilt: String
    public var status: String

    enum CodingKeys: String, CodingKey {
        case packageHash = "package_hash"
        case depsHash = "deps_hash"
        case lastBuilt = "last_built"
        case status
    }
}

public struct PackageEntry: Codable, Equatable {
    public var name: String
    public var relPath: String
    public var language: String
    public var buildCommands: [String]
    public var isStarlark: Bool
    public var declaredSrcs: [String]
    public var declaredDeps: [String]

    public init(
        name: String,
        relPath: String,
        language: String,
        buildCommands: [String],
        isStarlark: Bool = false,
        declaredSrcs: [String] = [],
        declaredDeps: [String] = []
    ) {
        self.name = name
        self.relPath = relPath
        self.language = language
        self.buildCommands = buildCommands
        self.isStarlark = isStarlark
        self.declaredSrcs = declaredSrcs
        self.declaredDeps = declaredDeps
    }

    enum CodingKeys: String, CodingKey {
        case name
        case relPath = "rel_path"
        case language
        case buildCommands = "build_commands"
        case isStarlark = "is_starlark"
        case declaredSrcs = "declared_srcs"
        case declaredDeps = "declared_deps"
    }
}

public struct BuildPlan: Codable, Equatable {
    public var schemaVersion: Int
    public var diffBase: String
    public var force: Bool
    public var affectedPackages: [String]?
    public var packages: [PackageEntry]
    public var dependencyEdges: [[String]]
    public var languagesNeeded: [String: Bool]

    public init(
        schemaVersion: Int,
        diffBase: String,
        force: Bool,
        affectedPackages: [String]?,
        packages: [PackageEntry],
        dependencyEdges: [[String]],
        languagesNeeded: [String: Bool]
    ) {
        self.schemaVersion = schemaVersion
        self.diffBase = diffBase
        self.force = force
        self.affectedPackages = affectedPackages
        self.packages = packages
        self.dependencyEdges = dependencyEdges
        self.languagesNeeded = languagesNeeded
    }

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case diffBase = "diff_base"
        case force
        case affectedPackages = "affected_packages"
        case packages
        case dependencyEdges = "dependency_edges"
        case languagesNeeded = "languages_needed"
    }
}

public struct CLIOptions: Equatable {
    public var root: String?
    public var force: Bool
    public var dryRun: Bool
    public var jobs: Int?
    public var language: String
    public var diffBase: String
    public var cacheFile: String
    public var emitPlan: String?
    public var planFile: String?
    public var detectLanguages: Bool
    public var validateBuildFiles: Bool
    public var help: Bool

    public init() {
        self.root = nil
        self.force = false
        self.dryRun = false
        self.jobs = nil
        self.language = "all"
        self.diffBase = "origin/main"
        self.cacheFile = ".build-cache.json"
        self.emitPlan = nil
        self.planFile = nil
        self.detectLanguages = false
        self.validateBuildFiles = false
        self.help = false
    }
}

public let allPackageLanguages = [
    "python",
    "ruby",
    "go",
    "typescript",
    "rust",
    "elixir",
    "lua",
    "perl",
    "swift",
    "java",
    "kotlin",
    "haskell",
    "wasm",
    "csharp",
    "fsharp",
    "dotnet",
]

public let allToolchains = [
    "python",
    "ruby",
    "go",
    "typescript",
    "rust",
    "elixir",
    "lua",
    "perl",
    "swift",
    "haskell",
    "dotnet",
]

public let allLanguages = allPackageLanguages

public let allToolchains = [
    "python",
    "ruby",
    "go",
    "typescript",
    "rust",
    "elixir",
    "lua",
    "perl",
    "swift",
    "java",
    "kotlin",
    "haskell",
    "dotnet",
]

public let sharedPrefixes: [String] = []

public func toolchainForPackageLanguage(_ language: String) -> String {
    switch language {
    case "wasm":
        return "rust"
    case "csharp", "fsharp", "dotnet":
        return "dotnet"
    default:
        return language
    }
}
