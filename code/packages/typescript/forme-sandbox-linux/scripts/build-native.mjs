import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform === "linux") {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  for (const [source, output] of [["launcher.c", "forme-sandbox-linux"], ["probe.c", "forme-sandbox-probe"]]) {
    const argumentsList = ["-std=c11", "-O2", "-Wall", "-Wextra", "-Werror"];
    if (source === "launcher.c") {
      const sha256Root = resolve(root, "../../c/sha256");
      argumentsList.push(
        `-I${resolve(sha256Root, "include")}`,
        resolve(sha256Root, "src/sha256.c"),
      );
    }
    argumentsList.push(resolve(root, "native", source), "-o", resolve(root, "native", output));
    const result = spawnSync("cc", argumentsList, { stdio: "inherit" });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}
