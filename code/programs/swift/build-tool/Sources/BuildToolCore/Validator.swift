import Foundation

public enum Validator {
    public static let ciManagedToolchainLanguages: Set<String> = [
        "python",
        "ruby",
        "typescript",
        "rust",
        "elixir",
        "lua",
        "perl",
        "java",
        "kotlin",
        "haskell",
    ]

    public static func validateCIFullBuildToolchains(repoRoot: String, packages: [BuildPackage]) -> String? {
        let ciPath = (repoRoot as NSString).appendingPathComponent(".github/workflows/ci.yml")
        guard let workflow = try? String(contentsOfFile: ciPath, encoding: .utf8) else {
            return nil
        }

        guard workflow.contains("Full build on main merge") else {
            return nil
        }

        let compactWorkflow = workflow.replacingOccurrences(
            of: #"\s+"#,
            with: "",
            options: .regularExpression
        )

        let languages = Set(packages.map(\.language)).intersection(ciManagedToolchainLanguages).sorted()
        var missingOutputBinding: [String] = []
        var missingMainForce: [String] = []

        for language in languages {
            let outputBinding = "needs_\(language):${{steps.toolchains.outputs.needs_\(language)}}"
            if !compactWorkflow.contains(outputBinding) {
                missingOutputBinding.append(language)
            }
            if !compactWorkflow.contains("needs_\(language)=true") {
                missingMainForce.append(language)
            }
        }

        if missingOutputBinding.isEmpty, missingMainForce.isEmpty {
            return nil
        }

        var parts: [String] = []
        if !missingOutputBinding.isEmpty {
            parts.append(
                "detect outputs for forced main full builds are not normalized through steps.toolchains for: \(missingOutputBinding.joined(separator: ", "))"
            )
        }
        if !missingMainForce.isEmpty {
            parts.append(
                "forced main full-build path does not explicitly enable toolchains for: \(missingMainForce.joined(separator: ", "))"
            )
        }

        return "\(ciPath.replacingOccurrences(of: "\\", with: "/")): \(parts.joined(separator: "; "))"
    }

    public static func validateBuildContracts(repoRoot: String, packages: [BuildPackage]) -> String? {
        var errors: [String] = []

        if let ciError = validateCIFullBuildToolchains(repoRoot: repoRoot, packages: packages) {
            errors.append(ciError)
        }

        errors.append(contentsOf: validateLuaIsolatedBuildFiles(packages: packages))
        errors.append(contentsOf: validatePerlBuildFiles(packages: packages))

        return errors.isEmpty ? nil : errors.joined(separator: "\n  - ")
    }

    static func validateLuaIsolatedBuildFiles(packages: [BuildPackage]) -> [String] {
        var errors: [String] = []

        for package in packages where package.language == "lua" {
            let selfRock = "coding-adventures-\((package.path as NSString).lastPathComponent.replacingOccurrences(of: "_", with: "-"))"
            var buildLines: [String: [String]] = [:]

            for buildPath in luaBuildFiles(packagePath: package.path) {
                let lines = readBuildLines(buildPath: buildPath)
                buildLines[(buildPath as NSString).lastPathComponent] = lines
                guard !lines.isEmpty else {
                    continue
                }

                if let foreignRemove = firstForeignLuaRemove(lines: lines, selfRock: selfRock) {
                    errors.append(
                        "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Lua BUILD removes unrelated rock \(foreignRemove); isolated package builds should only remove the package they are rebuilding"
                    )
                }

                let stateMachineIndex = firstLineContaining(lines: lines, needles: ["../state_machine", "..\\state_machine"])
                let directedGraphIndex = firstLineContaining(lines: lines, needles: ["../directed_graph", "..\\directed_graph"])
                if let stateMachineIndex, let directedGraphIndex, stateMachineIndex < directedGraphIndex {
                    errors.append(
                        "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Lua BUILD installs state_machine before directed_graph; isolated LuaRocks builds require directed_graph first"
                    )
                }

                if (hasGuardedLocalLuaInstall(lines: lines) ||
                    (((buildPath as NSString).lastPathComponent == "BUILD_windows") &&
                     hasLocalLuaSiblingInstall(lines: lines))) &&
                    !selfInstallDisablesDeps(lines: lines, selfRock: selfRock) {
                    errors.append(
                        "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Lua BUILD bootstraps sibling rocks but the final self-install does not pass --deps-mode=none or --no-manifest"
                    )
                }
            }

            let missingWindowsDeps = missingLuaSiblingInstalls(
                unixLines: buildLines["BUILD"] ?? [],
                windowsLines: buildLines["BUILD_windows"] ?? []
            )
            if !missingWindowsDeps.isEmpty {
                let buildPath = (package.path as NSString).appendingPathComponent("BUILD_windows")
                errors.append(
                    "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Lua BUILD_windows is missing sibling installs present in BUILD: \(missingWindowsDeps.joined(separator: ", "))"
                )
            }
        }

        return errors
    }

    static func validatePerlBuildFiles(packages: [BuildPackage]) -> [String] {
        var errors: [String] = []

        for package in packages where package.language == "perl" {
            for buildPath in luaBuildFiles(packagePath: package.path) {
                let lines = readBuildLines(buildPath: buildPath)
                if lines.contains(where: { line in
                    line.contains("cpanm") &&
                        line.contains("Test2::V0") &&
                        !line.contains("--notest")
                }) {
                    errors.append(
                        "\(buildPath.replacingOccurrences(of: "\\", with: "/")): Perl BUILD bootstraps Test2::V0 without --notest; isolated Windows installs can fail while installing the test framework itself"
                    )
                }
            }
        }

        return errors
    }

    static func luaBuildFiles(packagePath: String) -> [String] {
        guard let entries = try? FileManager.default.contentsOfDirectory(atPath: packagePath) else {
            return []
        }

        return entries
            .filter { $0.hasPrefix("BUILD") }
            .sorted()
            .map { (packagePath as NSString).appendingPathComponent($0) }
            .filter { FileManager.default.fileExists(atPath: $0) }
    }

    static func readBuildLines(buildPath: String) -> [String] {
        guard let contents = try? String(contentsOfFile: buildPath, encoding: .utf8) else {
            return []
        }

        return contents
            .split(separator: "\n", omittingEmptySubsequences: false)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty && !$0.hasPrefix("#") }
    }

    static func firstForeignLuaRemove(lines: [String], selfRock: String) -> String? {
        let pattern = try? NSRegularExpression(pattern: #"\bluarocks remove --force ([^ \t]+)"#)

        for line in lines {
            guard let pattern else { return nil }
            let range = NSRange(line.startIndex..<line.endIndex, in: line)
            guard let match = pattern.firstMatch(in: line, options: [], range: range),
                  let targetRange = Range(match.range(at: 1), in: line)
            else {
                continue
            }

            let target = String(line[targetRange])
            if target != selfRock {
                return target
            }
        }

        return nil
    }

    static func firstLineContaining(lines: [String], needles: [String]) -> Int? {
        for (index, line) in lines.enumerated() where needles.contains(where: { line.contains($0) }) {
            return index
        }
        return nil
    }

    static func hasGuardedLocalLuaInstall(lines: [String]) -> Bool {
        lines.contains { line in
            line.contains("luarocks show ") && (line.contains("../") || line.contains("..\\"))
        }
    }

    static func hasLocalLuaSiblingInstall(lines: [String]) -> Bool {
        !luaSiblingInstallDirs(lines: lines).isEmpty
    }

    static func selfInstallDisablesDeps(lines: [String], selfRock: String) -> Bool {
        lines.contains { line in
            line.contains("luarocks make") &&
                line.contains(selfRock) &&
                (line.contains("--deps-mode=none") ||
                    line.contains("--deps-mode none") ||
                    line.contains("--no-manifest"))
        }
    }

    static func missingLuaSiblingInstalls(unixLines: [String], windowsLines: [String]) -> [String] {
        let windowsDeps = Set(luaSiblingInstallDirs(lines: windowsLines))
        return luaSiblingInstallDirs(lines: unixLines).filter { !windowsDeps.contains($0) }
    }

    static func luaSiblingInstallDirs(lines: [String]) -> [String] {
        let pattern = try? NSRegularExpression(pattern: #"\bcd\s+([.][.][\\/][^ \t\r\n&()]+)"#)
        var deps = Set<String>()

        for line in lines {
            guard line.contains("luarocks make"), let pattern else {
                continue
            }

            let range = NSRange(line.startIndex..<line.endIndex, in: line)
            guard let match = pattern.firstMatch(in: line, options: [], range: range),
                  let depRange = Range(match.range(at: 1), in: line)
            else {
                continue
            }

            deps.insert(String(line[depRange]).replacingOccurrences(of: "\\", with: "/"))
        }

        return deps.sorted()
    }
}
