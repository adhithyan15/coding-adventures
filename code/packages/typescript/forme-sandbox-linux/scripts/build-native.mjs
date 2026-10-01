import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform === "linux") {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  for (const [source, output] of [["launcher.c", "forme-sandbox-linux"], ["probe.c", "forme-sandbox-probe"]]) {
    const result = spawnSync("cc", ["-std=c11", "-O2", "-Wall", "-Wextra", "-Werror",
      resolve(root, "native", source), "-o", resolve(root, "native", output)], { stdio: "inherit" });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}
