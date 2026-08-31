#!/usr/bin/env node

// npm invokes bin entries with Node, which cannot execute the repository's
// TypeScript-first packages directly. Register the same lightweight loader
// used by site development, then hand control to the testable CLI module.
import { register } from "tsx/esm/api";

const unregister = register();
try {
  const { main } = await import("../src/bin.ts");
  await main();
} finally {
  unregister();
}
