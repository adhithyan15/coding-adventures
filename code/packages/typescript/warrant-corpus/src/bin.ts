/**
 * `node --experimental-strip-types src/bin.ts <command> ...`
 *
 * The thin process entry: it runs {@link main} and exits with its status. It
 * is the file the `sabotage` command re-executes, once per negative control.
 */

import { main } from "./cli.ts";

process.exitCode = await main(process.argv.slice(2));
