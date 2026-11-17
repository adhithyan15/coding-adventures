#!/usr/bin/env node
import process from "node:process";
import { runCli } from "../dist/cli.js";

const exitCode = runCli(process.argv.slice(2));
process.exit(exitCode);
