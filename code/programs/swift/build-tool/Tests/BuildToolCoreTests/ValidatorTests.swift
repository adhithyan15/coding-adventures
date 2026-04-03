import BuildToolCore
import Foundation
import Testing

struct ValidatorTests {
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
