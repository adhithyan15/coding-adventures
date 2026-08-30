import { readdirSync, readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import {
  CANONICAL_TOOLCHAINS,
  ToolchainSnapshotError,
  evaluateToolchainSnapshot,
  parseExtraToolchains,
  type ToolchainPackageSnapshot,
} from "../src/toolchain-detection.js";

const fixtureDirectory = new URL(
  "../../../../specs/fixtures/build-tool-v1/cases/",
  import.meta.url,
);

const EXPECTED_FIXTURES = [
  "toolchain-detection-affected-only.json",
  "toolchain-detection-crlf-grammar.json",
  "toolchain-detection-declarations.json",
  "toolchain-detection-empty.json",
  "toolchain-detection-force-full.json",
  "toolchain-detection-null-all.json",
  "toolchain-detection-platform-darwin.json",
  "toolchain-detection-platform-linux.json",
  "toolchain-detection-platform-windows.json",
  "toolchain-detection-shared.json",
  "toolchain-detection-unsupported.json",
] as const;

interface NeutralFixture {
  id: string;
  input: {
    options: {
      platform: string;
      force_full: boolean;
      packages: Array<{
        name: string;
        language: string;
        build_files: Record<string, string>;
      }>;
      scheduled_packages: string[] | null;
      forced_toolchains: string[];
    };
  };
  expected: {
    outcome: "ok" | "error";
    result: { toolchains?: Record<string, boolean> };
    diagnostics: Array<{
      code: "TOOLCHAIN_UNSUPPORTED";
      severity: "error";
      package?: string;
    }>;
  };
}

function packageSnapshot(
  overrides: Partial<ToolchainPackageSnapshot> = {},
): ToolchainPackageSnapshot {
  return {
    name: "rust/app",
    language: "rust",
    buildFiles: { BUILD: "" },
    ...overrides,
  };
}

function expectSnapshotError(
  action: () => unknown,
  code: ToolchainSnapshotError["code"],
): void {
  try {
    action();
    throw new Error(`expected ${code}`);
  } catch (error: unknown) {
    expect(error).toBeInstanceOf(ToolchainSnapshotError);
    expect((error as ToolchainSnapshotError).code).toBe(code);
  }
}

describe("process-free TypeScript toolchain declaration boundary", () => {
  it("independently consumes every neutral toolchain fixture", () => {
    const fixtureNames = readdirSync(fixtureDirectory)
      .filter((name) => /^toolchain-detection-.*\.json$/u.test(name))
      .sort();
    expect(fixtureNames).toEqual(EXPECTED_FIXTURES);

    for (const fixtureName of fixtureNames) {
      const fixture = JSON.parse(
        readFileSync(new URL(fixtureName, fixtureDirectory), "utf8"),
      ) as NeutralFixture;
      const options = fixture.input.options;
      const actual = evaluateToolchainSnapshot({
        platform: options.platform,
        forceFull: options.force_full,
        packages: options.packages.map((entry) => ({
          name: entry.name,
          language: entry.language,
          buildFiles: entry.build_files,
        })),
        scheduledPackages: options.scheduled_packages,
        forcedToolchains: options.forced_toolchains,
      });

      expect(actual.outcome, fixture.id).toBe(fixture.expected.outcome);
      expect(actual.toolchains, fixture.id).toEqual(
        fixture.expected.result.toolchains ?? {},
      );
      expect(actual.diagnostics, fixture.id).toEqual(
        fixture.expected.diagnostics,
      );
    }
  });

  it("meters exact UTF-8 byte and logical-line ceilings before splitting", () => {
    for (const content of ["x".repeat(65_536), "é".repeat(32_768)]) {
      expect(
        evaluateToolchainSnapshot({
          platform: "linux",
          forceFull: false,
          packages: [packageSnapshot({ buildFiles: { BUILD: content } })],
          scheduledPackages: null,
          forcedToolchains: [],
        }).outcome,
      ).toBe("ok");
    }

    for (const content of ["x".repeat(65_537), "é".repeat(32_769)]) {
      expectSnapshotError(
        () =>
          evaluateToolchainSnapshot({
            platform: "linux",
            forceFull: false,
            packages: [packageSnapshot({ buildFiles: { BUILD: content } })],
            scheduledPackages: null,
            forcedToolchains: [],
          }),
        "BUILD_FRONT_TOO_LARGE",
      );
      expect(parseExtraToolchains(content)).toEqual([]);
    }

    expect(
      evaluateToolchainSnapshot({
        platform: "linux",
        forceFull: false,
        packages: [
          packageSnapshot({ buildFiles: { BUILD: "\n".repeat(4_095) } }),
        ],
        scheduledPackages: null,
        forcedToolchains: [],
      }).outcome,
    ).toBe("ok");
    const tooManyLines = "\n".repeat(4_096);
    expectSnapshotError(
      () =>
        evaluateToolchainSnapshot({
          platform: "linux",
          forceFull: false,
          packages: [
            packageSnapshot({ buildFiles: { BUILD: tooManyLines } }),
          ],
          scheduledPackages: null,
          forcedToolchains: [],
        }),
      "BUILD_FRONT_TOO_MANY_LINES",
    );
    expect(parseExtraToolchains(tooManyLines)).toEqual([]);
  });

  it("meters every front and the aggregate before scheduling", () => {
    const exactPackages = Array.from({ length: 16 }, (_, index) =>
      packageSnapshot({
        name: `rust/package-${index}`,
        buildFiles: { BUILD: "x".repeat(65_536) },
      }),
    );
    expect(
      evaluateToolchainSnapshot({
        platform: "linux",
        forceFull: false,
        packages: exactPackages,
        scheduledPackages: [],
        forcedToolchains: [],
      }).outcome,
    ).toBe("ok");

    expectSnapshotError(
      () =>
        evaluateToolchainSnapshot({
          platform: "linux",
          forceFull: false,
          packages: [
            ...exactPackages,
            packageSnapshot({
              name: "rust/package-16",
              buildFiles: { BUILD: "x".repeat(65_536) },
            }),
          ],
          scheduledPackages: [],
          forcedToolchains: [],
        }),
      "BUILD_SNAPSHOT_TOO_LARGE",
    );

    expectSnapshotError(
      () =>
        evaluateToolchainSnapshot({
          platform: "linux",
          forceFull: false,
          packages: [
            ...exactPackages,
            packageSnapshot({
              name: "rust/package-16",
              buildFiles: { BUILD: "x" },
            }),
            packageSnapshot({
              name: "rust/package-17",
              buildFiles: { BUILD: "x".repeat(65_537) },
            }),
          ],
          scheduledPackages: [],
          forcedToolchains: [],
        }),
      "BUILD_SNAPSHOT_TOO_LARGE",
    );
  });

  it("bounds direct-caller collections and identifier strings", () => {
    expectSnapshotError(
      () =>
        evaluateToolchainSnapshot({
          platform: "linux",
          forceFull: false,
          packages: Array.from({ length: 4_097 }, (_, index) =>
            packageSnapshot({ name: `rust/package-${index}` }),
          ),
          scheduledPackages: [],
          forcedToolchains: [],
        }),
      "SNAPSHOT_CARDINALITY_EXCEEDED",
    );

    const tooManyFronts = {
      BUILD: "",
      BUILD_windows: "",
      BUILD_mac: "",
      BUILD_linux: "",
      BUILD_mac_and_linux: "",
      BUILD_extra: "",
    };
    expectSnapshotError(
      () =>
        evaluateToolchainSnapshot({
          platform: "linux",
          forceFull: false,
          packages: [packageSnapshot({ buildFiles: tooManyFronts })],
          scheduledPackages: [],
          forcedToolchains: [],
        }),
      "SNAPSHOT_CARDINALITY_EXCEEDED",
    );

    for (const options of [
      {
        scheduledPackages: Array.from(
          { length: 4_097 },
          (_, index) => `rust/package-${index}`,
        ),
        forcedToolchains: [],
      },
      {
        scheduledPackages: [],
        forcedToolchains: Array.from({ length: 17 }, () => "rust"),
      },
    ]) {
      expectSnapshotError(
        () =>
          evaluateToolchainSnapshot({
            platform: "linux",
            forceFull: false,
            packages: [],
            ...options,
          }),
        "SNAPSHOT_CARDINALITY_EXCEEDED",
      );
    }

    for (const snapshot of [
      packageSnapshot({ name: `rust/${"x".repeat(236)}` }),
      packageSnapshot({ language: `r${"x".repeat(64)}` }),
    ]) {
      expectSnapshotError(
        () =>
          evaluateToolchainSnapshot({
            platform: "linux",
            forceFull: false,
            packages: [snapshot],
            scheduledPackages: [],
            forcedToolchains: [],
          }),
        "SNAPSHOT_STRING_INVALID",
      );
    }

    for (const options of [
      { scheduledPackages: ["not-a-package"], forcedToolchains: [] },
      { scheduledPackages: [], forcedToolchains: ["not_a_language"] },
    ]) {
      expectSnapshotError(
        () =>
          evaluateToolchainSnapshot({
            platform: "linux",
            forceFull: false,
            packages: [],
            ...options,
          }),
        "SNAPSHOT_STRING_INVALID",
      );
    }

    const sparsePackages = new Array<ToolchainPackageSnapshot>(1);
    expectSnapshotError(
      () =>
        evaluateToolchainSnapshot({
          platform: "linux",
          forceFull: false,
          packages: sparsePackages,
          scheduledPackages: [],
          forcedToolchains: [],
        }),
      "SNAPSHOT_INVALID",
    );

    for (const options of [
      {
        packages: [packageSnapshot({ buildFiles: { BUILD_linux: "" } })],
        scheduledPackages: [],
        forcedToolchains: [],
      },
      {
        packages: [],
        scheduledPackages: ["rust/app", "rust/app"],
        forcedToolchains: [],
      },
      {
        packages: [],
        scheduledPackages: [],
        forcedToolchains: ["rust", "rust"],
      },
    ]) {
      expectSnapshotError(
        () =>
          evaluateToolchainSnapshot({
            platform: "linux",
            forceFull: false,
            ...options,
          }),
        "SNAPSHOT_INVALID",
      );
    }
  });

  it("keeps declaration grammar byte-exact across CRLF and lone CR", () => {
    expect(
      parseExtraToolchains(
        "  # needs-toolchain: python  \r\n\t# needs-toolchain:\tjava\t\r\n",
      ),
    ).toEqual(["python", "java"]);
    expect(parseExtraToolchains("# needs-toolchain: python\r")).toEqual([]);
    expect(parseExtraToolchains("# needs-toolchain: lua\r  ")).toEqual([]);
    expect(parseExtraToolchains("# needs-toolchain: perl\r\t\n")).toEqual([]);
    expect(parseExtraToolchains("# needs-toolchain: swift\r\r\n")).toEqual([]);
  });

  it("stably deduplicates only exact canonical declarations", () => {
    expect(
      parseExtraToolchains(
        [
          "# needs-toolchain: python",
          "# needs-toolchain:\tjava",
          "# needs-toolchain: python",
          "# needs-toolchain: Python",
          "# needs-toolchain:zig",
          "# needs-toolchain: java suffix",
        ].join("\n"),
      ),
    ).toEqual(["python", "java"]);
  });

  it("preserves empty-front precedence and caller-owned inputs", () => {
    const packages = [
      packageSnapshot({
        buildFiles: {
          BUILD: "# needs-toolchain: java\n",
          BUILD_windows: "",
        },
      }),
    ];
    const before = structuredClone(packages);

    const actual = evaluateToolchainSnapshot({
      platform: "windows",
      forceFull: false,
      packages,
      scheduledPackages: null,
      forcedToolchains: ["kotlin"],
    });

    expect(actual.toolchains.rust).toBe(true);
    expect(actual.toolchains.kotlin).toBe(true);
    expect(actual.toolchains.java).toBe(false);
    expect(packages).toEqual(before);
  });

  it("keeps null-all and empty-none schedules distinct", () => {
    const packages = [packageSnapshot()];
    const all = evaluateToolchainSnapshot({
      platform: "linux",
      forceFull: false,
      packages,
      scheduledPackages: null,
      forcedToolchains: [],
    });
    const none = evaluateToolchainSnapshot({
      platform: "linux",
      forceFull: false,
      packages,
      scheduledPackages: [],
      forcedToolchains: [],
    });

    expect(all.toolchains.rust).toBe(true);
    expect(Object.values(none.toolchains).every((enabled) => !enabled)).toBe(
      true,
    );
  });

  it("returns fresh frozen complete maps in canonical order", () => {
    expect([...CANONICAL_TOOLCHAINS].sort()).toEqual(CANONICAL_TOOLCHAINS);
    expect(CANONICAL_TOOLCHAINS).toHaveLength(16);

    const options = {
      platform: "linux",
      forceFull: false,
      packages: [packageSnapshot()],
      scheduledPackages: null,
      forcedToolchains: [],
    } as const;
    const first = evaluateToolchainSnapshot(options);
    const second = evaluateToolchainSnapshot(options);

    expect(first.toolchains).not.toBe(second.toolchains);
    expect(Object.keys(second.toolchains)).toEqual(CANONICAL_TOOLCHAINS);
    expect(Object.isFrozen(first.toolchains)).toBe(true);
    expect(Object.isFrozen(first.diagnostics)).toBe(true);
    expect(Object.isFrozen(first)).toBe(true);
    expect(Reflect.set(first.toolchains, "zig", true)).toBe(false);
  });

  it("keeps unsupported diagnostics and their precedence stable", () => {
    const unsupportedPackage = evaluateToolchainSnapshot({
      platform: "linux",
      forceFull: true,
      packages: [packageSnapshot({ name: "zig/app", language: "zig" })],
      scheduledPackages: null,
      forcedToolchains: [],
    });
    expect(unsupportedPackage.diagnostics).toEqual([
      {
        code: "TOOLCHAIN_UNSUPPORTED",
        severity: "error",
        package: "zig/app",
      },
    ]);

    const packageBeforeForced = evaluateToolchainSnapshot({
      platform: "linux",
      forceFull: false,
      packages: [packageSnapshot({ name: "zig/app", language: "zig" })],
      scheduledPackages: null,
      forcedToolchains: ["zig"],
    });
    expect(packageBeforeForced.diagnostics).toEqual(
      unsupportedPackage.diagnostics,
    );

    const unsupportedForced = evaluateToolchainSnapshot({
      platform: "linux",
      forceFull: false,
      packages: [packageSnapshot()],
      scheduledPackages: [],
      forcedToolchains: ["zig"],
    });
    expect(unsupportedForced.diagnostics).toEqual([
      { code: "TOOLCHAIN_UNSUPPORTED", severity: "error" },
    ]);
  });

  it("rejects invalid platforms and force-full schedules before shortcuts", () => {
    for (const platform of ["solaris", "win32"]) {
      expectSnapshotError(
        () =>
          evaluateToolchainSnapshot({
            platform,
            forceFull: true,
            packages: [],
            scheduledPackages: null,
            forcedToolchains: [],
          }),
        "PLATFORM_UNSUPPORTED",
      );
    }
    expectSnapshotError(
      () =>
        evaluateToolchainSnapshot({
          platform: "linux",
          forceFull: true,
          packages: [],
          scheduledPackages: [],
          forcedToolchains: [],
        }),
      "FORCE_FULL_SCHEDULE_INVALID",
    );
  });
});
