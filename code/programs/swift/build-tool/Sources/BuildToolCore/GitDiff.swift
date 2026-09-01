import Foundation

public enum GitDiff {
    public static func getChangedFiles(repoRoot: String, diffBase: String = "origin/main") -> [String] {
        if let paths = try? runGitPaths(
            arguments: ["diff", "--name-only", "-z", "\(diffBase)...HEAD"],
            cwd: repoRoot
        ), !paths.isEmpty {
            return paths
        }

        if let paths = try? runGitPaths(
            arguments: ["diff", "--name-only", "-z", diffBase, "HEAD"],
            cwd: repoRoot
        ), !paths.isEmpty {
            return paths
        }

        return []
    }

    public static func mapFilesToPackages(
        changedFiles: [String],
        packagePaths: [String: String],
        repoRoot: String,
        packages: [BuildPackage]? = nil
    ) -> Set<String> {
        let packageByName = Dictionary(uniqueKeysWithValues: (packages ?? []).map { ($0.name, $0) })
        let normalizedRoot = normalize(repoRoot)
        var relativePackagePaths: [String: String] = [:]

        for (name, path) in packagePaths {
            let normalizedPath = normalize(path)
            if normalizedPath.hasPrefix(normalizedRoot + "/") {
                let relative = String(normalizedPath.dropFirst(normalizedRoot.count + 1))
                relativePackagePaths[name] = relative
            } else if normalizedPath == normalizedRoot {
                relativePackagePaths[name] = ""
            }
        }

        var changedPackages = Set<String>()
        var boundaryConsumers: [String: Set<String>] = [:]
        for (packageName, packageRoot) in relativePackagePaths {
            for input in Hasher.repositorySourceInputBoundaryRegistry.inputPaths(
                packageRoot: packageRoot
            ) {
                boundaryConsumers[input, default: []].insert(packageName)
            }
        }

        for file in changedFiles.map(normalize) {
            changedPackages.formUnion(boundaryConsumers[file] ?? [])
            for (packageName, packageRelativePath) in relativePackagePaths {
                guard file == packageRelativePath || file.hasPrefix(packageRelativePath + "/") else {
                    continue
                }

                if let package = packageByName[packageName],
                   package.isStarlark,
                   !package.declaredSrcs.isEmpty {
                    var relativeToPackage = file
                    if file.hasPrefix(packageRelativePath + "/") {
                        relativeToPackage = String(file.dropFirst(packageRelativePath.count + 1))
                    }
                    if relativeToPackage.hasPrefix("BUILD") {
                        changedPackages.insert(packageName)
                        break
                    }
                    if package.declaredSrcs.contains(where: { GlobMatch.matchPath($0, relativeToPackage) }) {
                        changedPackages.insert(packageName)
                    }
                    break
                }

                changedPackages.insert(packageName)
                break
            }
        }

        return changedPackages
    }

    static func getTrackedRegularFiles(
        repoRoot: String,
        exactPaths: [String]
    ) throws -> Set<String> {
        guard !exactPaths.isEmpty else {
            return []
        }
        guard exactPaths.count <= 256 else {
            throw BuildToolError.io("tracked repository input limit exceeded")
        }
        let expected = Set(exactPaths)
        guard expected.count == exactPaths.count else {
            throw BuildToolError.io("duplicate tracked repository input request")
        }
        let data = try runGitData(
            arguments: ["ls-files", "--stage", "-z", "--"] + exactPaths,
            cwd: repoRoot,
            maximumBytes: 1024 * 1024
        )
        guard data.isEmpty || data.last == 0 else {
            throw BuildToolError.io("unterminated tracked repository input evidence")
        }
        var result = Set<String>()
        var seenPaths = Set<String>()
        var identities: [String: String] = [:]
        for recordData in data.split(separator: 0, omittingEmptySubsequences: true) {
            guard let record = String(data: Data(recordData), encoding: .utf8),
                  let tab = record.firstIndex(of: "\t") else {
                throw BuildToolError.io("malformed tracked repository input evidence")
            }
            let metadata = record[..<tab].split(separator: " ")
            let path = String(record[record.index(after: tab)...])
            guard metadata.count == 3,
                  metadata[2] == "0",
                  expected.contains(path),
                  seenPaths.insert(path).inserted else {
                throw BuildToolError.io("invalid tracked repository input evidence")
            }
            try Hasher.validatePortablePath(path)
            try Hasher.registerPortableIdentity(path, in: &identities)
            if metadata[0] == "100644" || metadata[0] == "100755" {
                result.insert(path)
            } else if metadata[0] != "120000" && metadata[0] != "160000" {
                throw BuildToolError.io("unsupported tracked repository input mode")
            }
        }
        return result
    }

    private static func runGitPaths(arguments: [String], cwd: String) throws -> [String] {
        let data = try runGitData(
            arguments: arguments,
            cwd: cwd,
            maximumBytes: 16 * 1024 * 1024
        )
        guard data.isEmpty || data.last == 0 else {
            throw BuildToolError.io("unterminated Git diff output")
        }
        var result: [String] = []
        var seenPaths = Set<String>()
        var identities: [String: String] = [:]
        for pathData in data.split(separator: 0, omittingEmptySubsequences: true) {
            guard result.count < 100_000,
                  let path = String(data: Data(pathData), encoding: .utf8) else {
                throw BuildToolError.io("invalid Git diff output")
            }
            guard seenPaths.insert(path).inserted else {
                throw BuildToolError.io("duplicate Git diff path")
            }
            try Hasher.validatePortablePath(path)
            try Hasher.registerPortableIdentity(path, in: &identities)
            result.append(path)
        }
        return result
    }

    private static func runGitData(
        arguments: [String],
        cwd: String,
        maximumBytes: Int
    ) throws -> Data {
        let process = Process()
        #if os(Windows)
        guard let git = findWindowsGit() else {
            throw BuildToolError.io("git executable unavailable")
        }
        process.executableURL = git
        process.arguments = arguments
        #else
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["git"] + arguments
        #endif
        process.currentDirectoryURL = URL(fileURLWithPath: cwd)

        let output = Pipe()
        process.standardOutput = output
        process.standardError = output

        try process.run()
        var data = Data()
        while true {
            let remaining = maximumBytes - data.count
            let chunk = try output.fileHandleForReading.read(
                upToCount: min(64 * 1024, remaining + 1)
            ) ?? Data()
            if chunk.isEmpty {
                break
            }
            data.append(chunk)
            if data.count > maximumBytes {
                process.terminate()
                process.waitUntilExit()
                throw BuildToolError.io("git query exceeded its output limit")
            }
        }
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            throw BuildToolError.io("git query failed")
        }
        return data
    }

    private static func normalize(_ path: String) -> String {
        path.replacingOccurrences(of: "\\", with: "/")
    }

    #if os(Windows)
    private static func findWindowsGit() -> URL? {
        let environment = ProcessInfo.processInfo.environment
        guard let pathValue = environment.first(where: {
            $0.key.caseInsensitiveCompare("PATH") == .orderedSame
        })?.value else {
            return nil
        }
        for directory in pathValue.split(separator: ";", omittingEmptySubsequences: true) {
            let candidate = URL(fileURLWithPath: String(directory))
                .appendingPathComponent("git.exe")
            if FileManager.default.isExecutableFile(atPath: candidate.path) {
                return candidate
            }
        }
        return nil
    }
    #endif
}
