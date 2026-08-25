import BuildToolCore
import Foundation
import Testing

struct ValidatorTests {
    @Test(arguments: [
        "validation-tracked-artifacts-clean.json",
        "validation-tracked-artifacts-forbidden.json",
        "validation-tracked-artifacts-aliases.json",
        "validation-tracked-artifacts-invalid.json",
        "validation-tracked-artifacts-unicode-boundaries.json",
    ])
    func trackedArtifactValidatorConsumesSharedFixture(_ fixtureName: String) throws {
        let fixture = try loadSharedTrackedArtifactFixture(fixtureName)
        let snapshot = fixture.input.options.trackedArtifactSnapshot

        #expect(
            try Validator.validateTrackedArtifactSnapshot(
                unicodeVersion: snapshot.unicodeVersion,
                entries: snapshot.entries
            ) == fixture.expected.diagnostics
        )
    }

    @Test
    func trackedArtifactValidatorRejectsUnicodeVersionDrift() {
        #expect(throws: TrackedArtifactValidationError.self) {
            try Validator.validateTrackedArtifactSnapshot(
                unicodeVersion: "15.1.0",
                entries: []
            )
        }
    }

    @Test
    func trackedArtifactValidatorCountsUnicodeScalarsAtBoundary() throws {
        let valid = TrackedArtifactEntry(
            ordinal: 1,
            path: String(repeating: "😀", count: 512),
            entryKind: .regular
        )
        let tooLong = TrackedArtifactEntry(
            ordinal: 2,
            path: String(repeating: "😀", count: 513),
            entryKind: .regular
        )

        #expect(
            try Validator.validateTrackedArtifactSnapshot(entries: [valid]) == []
        )
        #expect(
            try Validator.validateTrackedArtifactSnapshot(entries: [tooLong]) == [
                trackedArtifactInvalidDiagnostic(
                    ordinal: 2,
                    entryKind: .regular,
                    problem: "TOO_LONG"
                ),
            ]
        )
    }

    @Test
    func trackedArtifactValidatorRedactsHostilePathsWithExactPrecedence() throws {
        let diagnostics = try Validator.validateTrackedArtifactSnapshot(entries: [
            TrackedArtifactEntry(ordinal: 1, path: "../bad<", entryKind: .regular),
            TrackedArtifactEntry(ordinal: 2, path: "safe/space /file", entryKind: .symlink),
            TrackedArtifactEntry(ordinal: 3, path: "safe/bad<name", entryKind: .reparse),
        ])

        #expect(diagnostics == [
            trackedArtifactInvalidDiagnostic(
                ordinal: 1,
                entryKind: .regular,
                problem: "UNSAFE_CHARACTER"
            ),
            trackedArtifactInvalidDiagnostic(
                ordinal: 3,
                entryKind: .reparse,
                problem: "UNSAFE_CHARACTER"
            ),
            trackedArtifactInvalidDiagnostic(
                ordinal: 2,
                entryKind: .symlink,
                problem: "TRAILING_DOT_OR_SPACE"
            ),
        ])
        #expect(diagnostics.allSatisfy { $0.path == "repository" })
    }

    @Test
    func trackedArtifactValidatorUsesPinnedUnicode17Behavior() throws {
        let outlinedNodeModules =
            "\u{1CCE3}\u{1CCE4}\u{1CCD9}\u{1CCDA}_" +
            "\u{1CCE2}\u{1CCE4}\u{1CCD9}\u{1CCEA}\u{1CCE1}\u{1CCDA}\u{1CCE8}"
        let diagnostics = try Validator.validateTrackedArtifactSnapshot(entries: [
            TrackedArtifactEntry(
                ordinal: 1,
                path: "\(outlinedNodeModules)/version.txt",
                entryKind: .regular
            ),
            TrackedArtifactEntry(
                ordinal: 2,
                path: "code/conın$.txt/file.cs",
                entryKind: .reparse
            ),
            TrackedArtifactEntry(
                ordinal: 3,
                path: "code/𐗉/file.rs",
                entryKind: .regular
            ),
        ])

        #expect(diagnostics == [
            TrackedArtifactDiagnostic(
                code: "TRACKED_ARTIFACT_FORBIDDEN",
                severity: "error",
                path: "\(outlinedNodeModules)/version.txt",
                details: TrackedArtifactDiagnosticDetails(
                    ordinal: 1,
                    entryKind: .regular,
                    problem: nil
                )
            ),
            trackedArtifactInvalidDiagnostic(
                ordinal: 3,
                entryKind: .regular,
                problem: "NON_NFC"
            ),
            trackedArtifactInvalidDiagnostic(
                ordinal: 2,
                entryKind: .reparse,
                problem: "RESERVED_BASENAME"
            ),
        ])
    }

    @Test
    func trackedArtifactValidatorSortsDetailsCanonicallyAsStrings() throws {
        let diagnostics = try Validator.validateTrackedArtifactSnapshot(entries: [
            TrackedArtifactEntry(ordinal: 2, path: "bad<path", entryKind: .regular),
            TrackedArtifactEntry(ordinal: 10, path: "bad<path", entryKind: .regular),
        ])

        #expect(diagnostics.map(\.details.ordinal) == [10, 2])
    }

    @Test
    func validatorFlagsMissingToolchainNormalization() throws {
        let root = try makeTempDirectory(label: "validator")
        defer { try? FileManager.default.removeItem(atPath: root) }

        try writeFile(
            (root as NSString).appendingPathComponent(".github/workflows/ci.yml"),
            """
            jobs:
              detect:
                outputs:
                  needs_python: ${{ steps.detect.outputs.needs_python }}
                  needs_elixir: ${{ steps.detect.outputs.needs_elixir }}
              build:
                steps:
                  - name: Full build on main merge
                    run: ./build-tool --force --validate-build-files --language all
            """
        )

        let packages = [
            BuildPackage(name: "python/actor", path: "/tmp/python/actor", language: "python"),
            BuildPackage(name: "elixir/actor", path: "/tmp/elixir/actor", language: "elixir"),
        ]

        let error = Validator.validateCIFullBuildToolchains(repoRoot: root, packages: packages)
        #expect(error != nil)
        #expect(error?.contains("python") == true)
        #expect(error?.contains("elixir") == true)
    }

    @Test
    func validatorAcceptsNormalizedWorkflow() throws {
        let root = try makeTempDirectory(label: "validator_ok")
        defer { try? FileManager.default.removeItem(atPath: root) }

        try writeFile(
            (root as NSString).appendingPathComponent(".github/workflows/ci.yml"),
            """
            jobs:
              detect:
                outputs:
                  needs_python: ${{ steps.toolchains.outputs.needs_python }}
                  needs_elixir: ${{ steps.toolchains.outputs.needs_elixir }}
                steps:
                  - name: Normalize toolchain requirements
                    id: toolchains
                    run: |
                      printf '%s\\n' \
                        'needs_python=true' \
                        'needs_elixir=true' >> "$GITHUB_OUTPUT"
              build:
                steps:
                  - name: Full build on main merge
                    run: ./build-tool --force --validate-build-files --language all
            """
        )

        let packages = [
            BuildPackage(name: "python/actor", path: "/tmp/python/actor", language: "python"),
            BuildPackage(name: "elixir/actor", path: "/tmp/elixir/actor", language: "elixir"),
        ]

        #expect(Validator.validateCIFullBuildToolchains(repoRoot: root, packages: packages) == nil)
    }

    @Test
    func validateBuildContractsFlagsLuaIsolatedBuildViolations() throws {
        let root = try makeTempDirectory(label: "validator_lua_bad")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let packagePath = (root as NSString).appendingPathComponent("code/packages/lua/problem_pkg")
        try writeFile(
            (packagePath as NSString).appendingPathComponent("BUILD"),
            """
            luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
            (cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
            (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
            luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
            """
        )

        let packages = [
            BuildPackage(name: "lua/problem_pkg", path: packagePath, language: "lua"),
        ]

        let error = Validator.validateBuildContracts(repoRoot: root, packages: packages)
        #expect(error?.contains("coding-adventures-branch-predictor") == true)
        #expect(error?.contains("state_machine before directed_graph") == true)
    }

    @Test
    func validateBuildContractsFlagsGuardedLuaInstallWithoutDepsMode() throws {
        let root = try makeTempDirectory(label: "validator_lua_guarded")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let packagePath = (root as NSString).appendingPathComponent("code/packages/lua/guarded_pkg")
        try writeFile(
            (packagePath as NSString).appendingPathComponent("BUILD"),
            """
            luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
            luarocks make --local coding-adventures-guarded-pkg-0.1.0-1.rockspec
            """
        )

        let packages = [
            BuildPackage(name: "lua/guarded_pkg", path: packagePath, language: "lua"),
        ]

        let error = Validator.validateBuildContracts(repoRoot: root, packages: packages)
        #expect(error?.contains("--deps-mode=none or --no-manifest") == true)
    }

    @Test
    func validateBuildContractsFlagsWindowsLuaSiblingDrift() throws {
        let root = try makeTempDirectory(label: "validator_lua_windows")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let packagePath = (root as NSString).appendingPathComponent("code/packages/lua/arm1_gatelevel")
        try writeFile(
            (packagePath as NSString).appendingPathComponent("BUILD"),
            """
            (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
            (cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
            (cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
            (cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
            luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
            """
        )
        try writeFile(
            (packagePath as NSString).appendingPathComponent("BUILD_windows"),
            """
            (cd ..\\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
            luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
            """
        )

        let packages = [
            BuildPackage(name: "lua/arm1_gatelevel", path: packagePath, language: "lua"),
        ]

        let error = Validator.validateBuildContracts(repoRoot: root, packages: packages)
        #expect(error?.contains("BUILD_windows is missing sibling installs present in BUILD") == true)
        #expect(error?.contains("../logic_gates") == true)
        #expect(error?.contains("../arithmetic") == true)
        #expect(error?.contains("--deps-mode=none or --no-manifest") == true)
    }

    @Test
    func validateBuildContractsFlagsPerlTest2BootstrapWithoutNotest() throws {
        let root = try makeTempDirectory(label: "validator_perl_test2")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let packagePath = (root as NSString).appendingPathComponent("code/packages/perl/draw-instructions-svg")
        try writeFile(
            (packagePath as NSString).appendingPathComponent("BUILD"),
            """
            cpanm --quiet Test2::V0
            prove -l -I../draw-instructions/lib -v t/
            """
        )

        let packages = [
            BuildPackage(name: "perl/draw-instructions-svg", path: packagePath, language: "perl"),
        ]

        let error = Validator.validateBuildContracts(repoRoot: root, packages: packages)
        #expect(error?.contains("Test2::V0 without --notest") == true)
    }

    @Test
    func validateBuildContractsAllowsSafeLuaPatterns() throws {
        let root = try makeTempDirectory(label: "validator_lua_safe")
        defer { try? FileManager.default.removeItem(atPath: root) }

        let packagePath = (root as NSString).appendingPathComponent("code/packages/lua/safe_pkg")
        try writeFile(
            (packagePath as NSString).appendingPathComponent("BUILD"),
            """
            luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
            luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
            luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
            luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
            """
        )
        try writeFile(
            (packagePath as NSString).appendingPathComponent("BUILD_windows"),
            """
            luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
            luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
            luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
            """
        )

        let packages = [
            BuildPackage(name: "lua/safe_pkg", path: packagePath, language: "lua"),
        ]

        #expect(Validator.validateBuildContracts(repoRoot: root, packages: packages) == nil)
    }
}

private func trackedArtifactInvalidDiagnostic(
    ordinal: Int,
    entryKind: TrackedArtifactEntryKind,
    problem: String
) -> TrackedArtifactDiagnostic {
    TrackedArtifactDiagnostic(
        code: "TRACKED_ARTIFACT_PATH_INVALID",
        severity: "error",
        path: "repository",
        details: TrackedArtifactDiagnosticDetails(
            ordinal: ordinal,
            entryKind: entryKind,
            problem: problem
        )
    )
}
