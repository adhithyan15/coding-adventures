import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform === "win32") {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  for (const [source, output, libraries] of [
    ["launcher.c", "forme-sandbox-windows.exe", ["advapi32.lib", "bcrypt.lib", "userenv.lib"]],
    ["probe.c", "forme-sandbox-probe.exe", ["ws2_32.lib"]],
  ]) {
    const result = spawnSync("cl.exe", ["/nologo", "/std:c11", "/O2", "/W4", "/WX",
      resolve(root, "native", source), `/Fe:${resolve(root, "native", output)}`, ...libraries], { stdio: "inherit" });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
}
