import fs from "node:fs";

process.on("SIGUSR2", () => process.kill(process.pid, "SIGKILL"));
if (process.platform === "win32") {
  fs.createReadStream(null, { fd: 4, autoClose: false }).once("data", () => process.exit(137));
}

const values = Object.fromEntries(process.argv.slice(2).map(argument => {
  const equals = argument.indexOf("=");
  return equals === -1 ? [argument, ""] : [argument.slice(0, equals), argument.slice(equals + 1)];
}));

if (process.env.FORME_SANDBOX_AMBIENT_SENTINEL !== undefined) process.exit(90);
if (values["--mode"] === "silent") {
  setTimeout(() => process.exit(91), 10_000);
} else if (values["--mode"] === "eof") {
  fs.closeSync(3);
  process.exit(92);
} else if (values["--mode"] === "stderr-eof") {
  process.stderr.write("native launcher diagnostic\n");
  fs.closeSync(3);
  process.exit(93);
} else if (values["--mode"] === "oversized") {
  fs.writeSync(3, "x".repeat(4097));
} else if (values["--mode"] === "trailing") {
  fs.writeSync(3, "{}\ntrailing");
} else if (values["--mode"] === "malformed") {
  fs.writeSync(3, "{\n");
} else if (values["--mode"] === "invalid-shape") {
  fs.writeSync(3, "[]\n");
} else if (values["--mode"] === "invalid-fields") {
  fs.writeSync(3, '{"protocol":2}\n');
} else {

  fs.writeSync(3, JSON.stringify({
    protocol: 1,
    provider: "forme-test-v1",
    manifestHash: values["--manifest-hash"],
    configSchemaHash: values["--schema-hash"] === "-" ? null : values["--schema-hash"],
    entryHash: values["--mode"] === "wrong-entry" ? "sha256:wrong" : values["--entry-hash"],
  }) + "\n");
  fs.closeSync(3);
  process.stdin.resume();
}
