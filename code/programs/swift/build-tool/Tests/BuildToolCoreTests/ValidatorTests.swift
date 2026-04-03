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
}
