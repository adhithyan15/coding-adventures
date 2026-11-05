import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const MAX_RESPONSE_BYTES = 64 * 1024 * 1024;

/** One pinned-parent, exact-set read. The Python boundary never exposes error bytes. */
export async function readAnchoredModalityTrack(
  root: string,
  language: string,
  names: readonly string[],
): Promise<Map<string, string>> {
  const script = fileURLToPath(new URL(
    "../../../../learning/human-languages/data/scripts/modality_owner_read.py",
    import.meta.url,
  ));
  const python = process.platform === "win32" ? "python" : "python3";
  const output = await new Promise<Buffer>((resolve, reject) => {
    const child = spawn(python, [script], { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
    const chunks: Buffer[] = [];
    let bytes = 0;
    let failed = false;
    child.stdout.on("data", (chunk: Buffer) => {
      bytes += chunk.length;
      if (bytes > MAX_RESPONSE_BYTES) {
        failed = true;
        child.kill();
      } else {
        chunks.push(chunk);
      }
    });
    // Python's stderr can include exception context; never forward it to CI.
    child.stderr.resume();
    child.stdin.on("error", () => { /* A failed child is handled by close/error. */ });
    child.on("error", (cause) => reject(new Error("modality anchored reader could not start", { cause })));
    child.on("close", (code) => {
      if (failed || code !== 0) {
        reject(new Error(`modality anchored reader failed for '${language}'`));
      } else {
        resolve(Buffer.concat(chunks));
      }
    });
    child.stdin.end(JSON.stringify({ root, language, names }));
  });

  let response: unknown;
  try {
    response = JSON.parse(output.toString("utf8"));
  } catch (cause) {
    throw new Error(`modality anchored reader returned invalid JSON for '${language}'`, { cause });
  }
  if (typeof response !== "object" || response === null || Array.isArray(response)) {
    throw new Error(`modality anchored reader returned invalid response for '${language}'`);
  }
  const files = (response as Record<string, unknown>)["files"];
  if (typeof files !== "object" || files === null || Array.isArray(files)) {
    throw new Error(`modality anchored reader omitted files for '${language}'`);
  }
  const found = new Map<string, string>();
  for (const name of names) {
    const text = (files as Record<string, unknown>)[name];
    if (!Object.hasOwn(files, name) || typeof text !== "string") {
      throw new Error(`modality anchored reader omitted '${language}.d/${name}'`);
    }
    found.set(name, text);
  }
  if (Object.keys(files).length !== found.size) {
    throw new Error(`modality anchored reader returned extra files for '${language}'`);
  }
  return found;
}
