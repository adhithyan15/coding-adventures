/**
 * @coding-adventures/forme-style-cli
 *
 * Node.js CLI wrapping `@coding-adventures/forme-style-orchestrator`
 * with file I/O.  Per FM03 §5.
 *
 * The npm `bin` entry is `forme-style` (see `bin/forme-style` →
 * `src/bin.ts`).  This module re-exports the testable `run(argv, io)`
 * function so programmatic callers can drive the CLI in-process
 * without spawning a subprocess.
 *
 * ```ts
 * import { run } from "@coding-adventures/forme-style-cli";
 *
 * const code = await run(["doc.json", "--target", "css"], {
 *   stdout: process.stdout,
 *   stderr: process.stderr,
 *   readFile: (p) => fs.promises.readFile(p, "utf8"),
 *   writeFile: (p, c) => fs.promises.writeFile(p, c, "utf8"),
 *   readStdin: () => Promise.resolve(""),
 * });
 * process.exit(code);
 * ```
 *
 * @module index
 */

export {
  run,
  EXIT_OK, EXIT_VALIDATOR_FAIL, EXIT_IO_OR_ARG_ERROR, EXIT_UNKNOWN_TARGET,
} from "./cli.js";
export type { CliIO } from "./cli.js";
