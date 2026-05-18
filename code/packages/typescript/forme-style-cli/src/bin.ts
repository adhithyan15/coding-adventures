#!/usr/bin/env node
/**
 * bin.ts — npm `bin` shim around `cli.ts`'s `run()`.
 *
 * Intentionally trivial.  Wires `process.argv` / `process.stdout` /
 * `process.stderr` / `node:fs` / `process.stdin` to the testable
 * `run(argv, io)` function and propagates the returned exit code.
 *
 * @module bin
 */

import { promises as fs } from "node:fs";
import { run } from "./cli.js";

async function readStdin(): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString("utf8");
}

run(process.argv.slice(2), {
  stdout: process.stdout,
  stderr: process.stderr,
  readFile: (p) => fs.readFile(p, "utf8"),
  writeFile: (p, c) => fs.writeFile(p, c, "utf8"),
  readStdin,
}).then((code) => {
  process.exitCode = code;
}, (err) => {
  // Belt-and-braces: if `run` itself throws an unexpected exception,
  // surface it as exit code 2 with the error visible.
  process.stderr.write(`forme-style: unexpected error: ${(err as Error).stack ?? String(err)}\n`);
  process.exitCode = 2;
});
