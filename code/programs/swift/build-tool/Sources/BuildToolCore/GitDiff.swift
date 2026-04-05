import Foundation

public enum GitDiff {
    public static func getChangedFiles(repoRoot: String, diffBase: String = "origin/main") -> [String] {
        if let output = try? runGit(arguments: ["diff", "--name-only", "\(diffBase)...HEAD"], cwd: repoRoot),
           !output.isEmpty {
            return output.split(whereSeparator: \.isNewline).map(String.init).filter { !$0.isEmpty }
        }

        if let output = try? runGit(arguments: ["diff", "--name-only", diffBase, "HEAD"], cwd: repoRoot),
           !output.isEmpty {
            return output.split(whereSeparator: \.isNewline).map(String.init).filter { !$0.isEmpty }
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

        for file in changedFiles.map(normalize) {
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

    private static func runGit(arguments: [String], cwd: String) throws -> String {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["git"] + arguments
        process.currentDirectoryURL = URL(fileURLWithPath: cwd)

        let stdout = Pipe()
        process.standardOutput = stdout
        let stderr = Pipe()
        process.standardError = stderr

        try process.run()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            throw BuildToolError.io(String(data: stderr.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? "git failed")
        }

        return String(data: stdout.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
    }

    private static func normalize(_ path: String) -> String {
        path.replacingOccurrences(of: "\\", with: "/")
    }
}
