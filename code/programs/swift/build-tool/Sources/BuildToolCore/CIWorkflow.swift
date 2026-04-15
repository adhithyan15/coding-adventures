import Foundation

public struct CIWorkflowChange {
    public var toolchains: Set<String>
    public var requiresFullRebuild: Bool

    public init(toolchains: Set<String> = [], requiresFullRebuild: Bool = false) {
        self.toolchains = toolchains
        self.requiresFullRebuild = requiresFullRebuild
    }
}

public enum CIWorkflow {
    public static let workflowPath = ".github/workflows/ci.yml"

    private static let toolchainMarkers: [String: [String]] = [
        "python": [
            "needs_python", "setup-python", "python-version", "setup-uv",
            "python --version", "uv --version", "pytest",
            "set up python", "install uv",
        ],
        "ruby": [
            "needs_ruby", "setup-ruby", "ruby-version", "bundler",
            "gem install bundler", "ruby --version", "bundle --version",
            "set up ruby", "install bundler",
        ],
        "go": ["needs_go", "setup-go", "go-version", "go version", "set up go"],
        "typescript": [
            "needs_typescript", "setup-node", "node-version", "npm install -g jest",
            "node --version", "npm --version", "set up node",
        ],
        "rust": [
            "needs_rust", "rust-toolchain", "cargo", "rustc", "tarpaulin",
            "wasm32-unknown-unknown", "set up rust", "install cargo-tarpaulin",
        ],
        "elixir": [
            "needs_elixir", "setup-beam", "elixir-version", "otp-version",
            "elixir --version", "mix --version", "set up elixir",
        ],
        "lua": [
            "needs_lua", "gh-actions-lua", "gh-actions-luarocks", "luarocks",
            "lua -v", "msvc", "set up lua", "set up luarocks",
        ],
        "perl": ["needs_perl", "cpanm", "perl --version", "install cpanm"],
        "haskell": [
            "needs_haskell", "haskell-actions/setup", "ghc-version", "cabal-version",
            "ghc --version", "cabal --version", "set up haskell",
        ],
        "java": [
            "needs_java", "setup-java", "java-version", "java --version",
            "temurin", "set up jdk", "set up gradle", "setup-gradle",
            "disable long-lived gradle services",
            "gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch",
        ],
        "kotlin": [
            "needs_kotlin", "setup-java", "java-version",
            "temurin", "set up jdk", "set up gradle", "setup-gradle",
            "disable long-lived gradle services",
            "gradle_opts", "org.gradle.daemon", "org.gradle.vfs.watch",
        ],
        "dotnet": [
            "needs_dotnet", "setup-dotnet", "dotnet-version", "dotnet --version",
            "set up .net",
        ],
    ]

    private static let unsafeMarkers = [
        "./build-tool",
        "build-tool.exe",
        "-detect-languages",
        "-emit-plan",
        "-force",
        "-plan-file",
        "-validate-build-files",
        "actions/checkout",
        "build-plan",
        "cancel-in-progress:",
        "concurrency:",
        "diff-base",
        "download-artifact",
        "event_name",
        "fetch-depth",
        "git fetch origin main",
        "git_ref",
        "is_main",
        "matrix:",
        "permissions:",
        "pr_base_ref",
        "pull_request:",
        "push:",
        "runs-on:",
        "strategy:",
        "upload-artifact",
    ]

    public static func analyzeChanges(repoRoot: String, diffBase: String) -> CIWorkflowChange {
        analyzePatch(fileDiff(repoRoot: repoRoot, diffBase: diffBase, relativePath: workflowPath))
    }

    public static func analyzePatch(_ patch: String) -> CIWorkflowChange {
        var toolchains = Set<String>()
        var hunk: [String] = []

        func flush() -> CIWorkflowChange? {
            let (hunkToolchains, unsafe) = classifyHunk(hunk)
            hunk.removeAll(keepingCapacity: true)
            if unsafe {
                return CIWorkflowChange(requiresFullRebuild: true)
            }
            toolchains.formUnion(hunkToolchains)
            return nil
        }

        for line in patch.split(separator: "\n", omittingEmptySubsequences: false).map(String.init) {
            if line.hasPrefix("@@") {
                if let result = flush() {
                    return result
                }
                continue
            }

            if line.hasPrefix("diff --git ")
                || line.hasPrefix("index ")
                || line.hasPrefix("--- ")
                || line.hasPrefix("+++ ")
            {
                continue
            }

            hunk.append(line)
        }

        if let result = flush() {
            return result
        }
        return CIWorkflowChange(toolchains: toolchains)
    }

    public static func sortedToolchains(_ toolchains: Set<String>) -> [String] {
        toolchains.sorted()
    }

    private static func classifyHunk(_ lines: [String]) -> (Set<String>, Bool) {
        var hunkToolchains = Set<String>()
        var changedToolchains = Set<String>()
        var changedLines: [String] = []

        for line in lines {
            guard !line.isEmpty, isDiffLine(line) else {
                continue
            }

            let content = String(line.dropFirst()).trimmingCharacters(in: .whitespacesAndNewlines)
            hunkToolchains.formUnion(detectToolchains(content))

            guard isChangedLine(line) else {
                continue
            }
            guard !content.isEmpty, !content.hasPrefix("#") else {
                continue
            }

            changedLines.append(content)
            changedToolchains.formUnion(detectToolchains(content))
        }

        guard !changedLines.isEmpty else {
            return ([], false)
        }

        var resolvedToolchains = changedToolchains
        if resolvedToolchains.isEmpty {
            guard hunkToolchains.count == 1 else {
                return ([], true)
            }
            resolvedToolchains = hunkToolchains
        }

        for content in changedLines {
            if touchesSharedCIBehavior(content) {
                return ([], true)
            }
            if !detectToolchains(content).isEmpty {
                continue
            }
            if isToolchainScopedStructuralLine(content) {
                continue
            }
            return ([], true)
        }

        return (resolvedToolchains, false)
    }

    private static func detectToolchains(_ content: String) -> Set<String> {
        let normalized = content.lowercased()
        return Set(
            toolchainMarkers.compactMap { toolchain, markers in
                markers.contains(where: { normalized.contains($0) }) ? toolchain : nil
            }
        )
    }

    private static func touchesSharedCIBehavior(_ content: String) -> Bool {
        let normalized = content.lowercased()
        return unsafeMarkers.contains(where: { normalized.contains($0) })
    }

    private static func isToolchainScopedStructuralLine(_ content: String) -> Bool {
        [
            "if:",
            "run:",
            "shell:",
            "with:",
            "env:",
            "{",
            "}",
            "else",
            "fi",
            "then",
            "printf ",
            "echo ",
            "curl ",
            "powershell ",
            "call ",
            "cd ",
        ].contains(where: { content.hasPrefix($0) })
    }

    private static func isDiffLine(_ line: String) -> Bool {
        line.hasPrefix(" ") || isChangedLine(line)
    }

    private static func isChangedLine(_ line: String) -> Bool {
        line.hasPrefix("+") || line.hasPrefix("-")
    }

    private static func fileDiff(repoRoot: String, diffBase: String, relativePath: String) -> String {
        let argLists = [
            ["diff", "--unified=0", "\(diffBase)...HEAD", "--", relativePath],
            ["diff", "--unified=0", diffBase, "HEAD", "--", relativePath],
        ]

        for args in argLists {
            if let output = try? runGit(args: args, repoRoot: repoRoot) {
                return output
            }
        }

        return ""
    }

    private static func runGit(args: [String], repoRoot: String) throws -> String {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["git"] + args
        process.currentDirectoryURL = URL(fileURLWithPath: repoRoot)

        let stdout = Pipe()
        process.standardOutput = stdout
        process.standardError = Pipe()

        try process.run()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            throw BuildToolError.io("git diff failed")
        }

        let data = stdout.fileHandleForReading.readDataToEndOfFile()
        return String(decoding: data, as: UTF8.self)
    }
}
