import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform === "darwin") {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  mkdirSync(resolve(root, "native"), { recursive: true });
  for (const [source, output, extra] of [
    ["launcher.c", "forme-sandbox-macos", ["-Wno-deprecated-declarations"]],
    ["probe.c", "forme-sandbox-probe", []],
  ]) {
    const result = spawnSync("xcrun", ["clang", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", ...extra,
      resolve(root, "native", source), "-o", resolve(root, "native", output)], { stdio: "inherit" });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}
