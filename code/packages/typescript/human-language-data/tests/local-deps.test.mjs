import { describe, expect, it } from "vitest";
import { npmCiInvocation } from "../local-deps.mjs";

describe("local dependency npm invocation", () => {
  it("runs npm's JavaScript CLI through Node on Windows", () => {
    expect(
      npmCiInvocation({
        platform: "win32",
        nodeExecutable: "C:\\Program Files\\nodejs\\node.exe",
        npmExecutable: "C:\\Program Files\\nodejs\\node_modules\\npm\\bin\\npm-cli.js",
      }),
    ).toEqual({
      executable: "C:\\Program Files\\nodejs\\node.exe",
      args: [
        "C:\\Program Files\\nodejs\\node_modules\\npm\\bin\\npm-cli.js",
        "ci",
        "--silent",
      ],
    });
  });

  it("keeps the direct executable and fixed argv on POSIX", () => {
    expect(npmCiInvocation({ platform: "linux" })).toEqual({
      executable: "npm",
      args: ["ci", "--silent"],
    });
  });

  it("fails closed when Windows has no shell-free npm CLI path", () => {
    expect(() =>
      npmCiInvocation({
        platform: "win32",
        nodeExecutable: "node.exe",
        npmExecutable: "",
      }),
    ).toThrow(/npm_execpath is required.*without a shell on Windows/);
  });
});
