import Foundation

public enum Executor {
    public static func executeBuilds(
        packages: [BuildPackage],
        graph: DirectedGraph,
        cache: BuildCache,
        packageHashes: [String: String],
        depsHashes: [String: String],
        force: Bool = false,
        dryRun: Bool = false,
        maxJobs: Int? = nil,
        affectedSet: Set<String>? = nil
    ) throws -> [String: PackageBuildResult] {
        let packageByName = Dictionary(uniqueKeysWithValues: packages.map { ($0.name, $0) })
        let groups = try graph.independentGroups()

        var results: [String: PackageBuildResult] = [:]
        var failedPackages = Set<String>()
        let resultLock = NSLock()
        let cacheLock = NSLock()

        for group in groups {
            var toBuild: [BuildPackage] = []

            for name in group {
                guard let package = packageByName[name] else {
                    continue
                }

                let dependencyFailed = !graph.transitivePrerequisites(of: name).intersection(failedPackages).isEmpty
                if dependencyFailed {
                    results[name] = PackageBuildResult(packageName: name, status: .depSkipped)
                    continue
                }

                if let affectedSet, !affectedSet.contains(name) {
                    results[name] = PackageBuildResult(packageName: name, status: .skipped)
                    continue
                }

                let packageHash = packageHashes[name] ?? ""
                let depsHash = depsHashes[name] ?? ""

                if affectedSet == nil, !force, !cache.needsBuild(name: name, packageHash: packageHash, depsHash: depsHash) {
                    results[name] = PackageBuildResult(packageName: name, status: .skipped)
                    continue
                }

                if dryRun {
                    results[name] = PackageBuildResult(packageName: name, status: .wouldBuild)
                    continue
                }

                toBuild.append(package)
            }

            if toBuild.isEmpty || dryRun {
                continue
            }

            let queue = OperationQueue()
            queue.maxConcurrentOperationCount = maxJobs ?? min(toBuild.count, 8)

            for package in toBuild {
                queue.addOperation {
                    let result = runPackageBuild(package)
                    resultLock.lock()
                    results[package.name] = result
                    if result.status == .failed {
                        failedPackages.insert(package.name)
                    }
                    resultLock.unlock()

                    cacheLock.lock()
                    if result.status == .built {
                        cache.record(
                            name: package.name,
                            packageHash: packageHashes[package.name] ?? "",
                            depsHash: depsHashes[package.name] ?? "",
                            status: "success"
                        )
                    } else if result.status == .failed {
                        cache.record(
                            name: package.name,
                            packageHash: packageHashes[package.name] ?? "",
                            depsHash: depsHashes[package.name] ?? "",
                            status: "failed"
                        )
                    }
                    cacheLock.unlock()
                }
            }

            queue.waitUntilAllOperationsAreFinished()
        }

        return results
    }

    public static func runPackageBuild(_ package: BuildPackage, timeout: TimeInterval = 600) -> PackageBuildResult {
        let start = Date()
        var stdoutParts: [String] = []
        var stderrParts: [String] = []

        for command in package.buildCommands {
            let (terminationStatus, stdout, stderr, timedOut) = runCommand(
                command,
                cwd: package.path,
                timeout: timeout
            )
            stdoutParts.append(stdout)
            stderrParts.append(stderr)

            if timedOut {
                return PackageBuildResult(
                    packageName: package.name,
                    status: .failed,
                    duration: Date().timeIntervalSince(start),
                    stdout: stdoutParts.joined(),
                    stderr: stderrParts.joined() + "\nBuild command timed out after \(Int(timeout))s",
                    returnCode: 124
                )
            }

            if terminationStatus != 0 {
                return PackageBuildResult(
                    packageName: package.name,
                    status: .failed,
                    duration: Date().timeIntervalSince(start),
                    stdout: stdoutParts.joined(),
                    stderr: stderrParts.joined(),
                    returnCode: terminationStatus
                )
            }
        }

        return PackageBuildResult(
            packageName: package.name,
            status: .built,
            duration: Date().timeIntervalSince(start),
            stdout: stdoutParts.joined(),
            stderr: stderrParts.joined()
        )
    }

    private static func runCommand(_ command: String, cwd: String, timeout: TimeInterval) -> (Int32, String, String, Bool) {
        let process = Process()
        #if os(Windows)
        process.executableURL = URL(fileURLWithPath: "C:/Windows/System32/cmd.exe")
        process.arguments = ["/c", command]
        #else
        process.executableURL = URL(fileURLWithPath: "/bin/sh")
        process.arguments = ["-c", command]
        #endif
        process.currentDirectoryURL = URL(fileURLWithPath: cwd)

        let stdoutPipe = Pipe()
        let stderrPipe = Pipe()
        process.standardOutput = stdoutPipe
        process.standardError = stderrPipe

        let group = DispatchGroup()
        group.enter()

        do {
            try process.run()
        } catch {
            group.leave()
            return (1, "", String(describing: error), false)
        }

        DispatchQueue.global().async {
            process.waitUntilExit()
            group.leave()
        }

        let timedOut = group.wait(timeout: .now() + timeout) == .timedOut
        if timedOut {
            process.terminate()
        }

        let stdout = String(data: stdoutPipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        let stderr = String(data: stderrPipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
        return (process.terminationStatus, stdout, stderr, timedOut)
    }
}
