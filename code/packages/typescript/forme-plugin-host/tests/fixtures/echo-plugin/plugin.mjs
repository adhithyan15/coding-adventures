import process from "node:process";

const stageId = process.argv[2] ?? "echo";
const mismatch = process.argv.includes("--mismatch");
const announceMismatch = process.argv.includes("--announce-mismatch");
const invalidRunner = process.argv.includes("--invalid-runner");
const ignoreDispose = process.argv.includes("--ignore-dispose");
const ignoreInit = process.argv.includes("--ignore-init");
const streamStage = stageId === "stream";
const reduceStage = stageId === "reduce";
if (process.argv.includes("--ignore-term")) process.on("SIGTERM", () => {});
let buffer = Buffer.alloc(0);
let nextRequestId = -1;
const pending = new Map();
const inputStreams = new Map();

function encode(value) {
  if (value instanceof Uint8Array) {
    return { $forme: "bytes", base64: Buffer.from(value).toString("base64") };
  }
  if (Array.isArray(value)) return value.map(encode);
  if (value && typeof value === "object") {
    if (Object.hasOwn(value, "$forme")) {
      return { $forme: "escaped-object", entries: Object.entries(value).map(([k, v]) => [k, encode(v)]) };
    }
    return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, encode(v)]));
  }
  return value;
}

function decode(value) {
  if (Array.isArray(value)) return value.map(decode);
  if (value && typeof value === "object") {
    if (value.$forme === "bytes" && Object.keys(value).length === 2) {
      return Uint8Array.from(Buffer.from(value.base64, "base64"));
    }
    if (value.$forme === "escaped-object" && Object.keys(value).length === 2) {
      return Object.fromEntries(value.entries.map(([k, v]) => [k, decode(v)]));
    }
    return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, decode(v)]));
  }
  return value;
}

function send(message) {
  const payload = Buffer.from(JSON.stringify(encode(message)), "utf8");
  process.stdout.write(Buffer.concat([
    Buffer.from(`Content-Length: ${payload.length}\r\n\r\n`, "ascii"),
    payload,
  ]));
}

function respond(id, result) {
  send({ jsonrpc: "2.0", id, result });
}

function request(method, params) {
  const id = nextRequestId--;
  send({ jsonrpc: "2.0", id, method, params });
  return new Promise((resolve) => pending.set(id, resolve));
}

async function handle(message) {
  if (Object.hasOwn(message, "id") && !message.method) {
    const resolve = pending.get(message.id);
    if (resolve) {
      pending.delete(message.id);
      resolve(message);
    }
    return;
  }

  if (message.method === "handshake") {
    const p = message.params;
    respond(message.id, {
      pluginName: mismatch ? "@example/wrong" : p.pluginName,
      pluginVersion: p.pluginVersion,
      apiVersion: p.apiVersion,
      protocolVersion: p.protocolVersion,
      runner: invalidRunner ? 7 : "forme-fixture-runner",
      runnerVersion: "1.0.0",
    });
    return;
  }

  if (message.method === "announce") {
    respond(message.id, {
      stage: {
        id: announceMismatch ? "wrong" : stageId,
        consumes: streamStage || reduceStage ? "Stream<ContentNode>" : "ContentNode",
        produces: streamStage ? "Stream<ContentNode>" : "ContentNode",
        capabilities: ["storage:read", "env:FIXTURE_SECRET"],
        configSchemaHash: stageId === "echo"
          ? "sha256:31259dcce1aec6aac8a7352eda5cc970d4fed2a21e1a0b6bdd8a3a139a3252d7"
          : null,
      },
    });
    return;
  }

  if (message.method === "stage.init") {
    if (ignoreInit) return;
    respond(message.id, null);
    return;
  }

  if (message.method === "stage.run") {
    const input = message.params.input;
    if (input?.crash === true) process.exit(17);
    if (input?.hang === true) return;
    if (input?.malformedSingle === true) {
      respond(message.id, { kind: "stream", value: input });
      return;
    }
    if (input?.remoteErrorCode) {
      send({
        jsonrpc: "2.0",
        id: message.id,
        error: {
          code: input.remoteErrorCode,
          message: "fixture remote error",
          data: input.remoteErrorData,
        },
      });
      return;
    }
    if (input?.stderr === true) process.stderr.write("fixture stderr");
    if (input?.badStreamNotification === true) {
      send({ jsonrpc: "2.0", method: "stream.value", params: { streamId: "bad", value: null } });
      return;
    }
    if (input?.inactiveStreamNotification === true) {
      send({ jsonrpc: "2.0", method: "stream.value", params: { streamId: 999, value: null } });
      return;
    }
    if (input?.badCapabilityStreamId === true) {
      send({ jsonrpc: "2.0", method: "stream.start", params: { streamId: "bad" } });
      return;
    }
    if (input?.inactiveCapabilityStream === true) {
      send({ jsonrpc: "2.0", method: "stream.start", params: { streamId: 999 } });
      return;
    }
    if (input?.badCapabilityCancel === true || input?.inactiveCapabilityCancel === true) {
      const reply = await request("stream.cancel", {
        streamId: message.params.streamId,
        capabilityStreamId: input.badCapabilityCancel ? "bad" : 999,
      });
      respond(message.id, {
        kind: "single",
        value: { ...input, cancelCode: reply?.error?.code ?? null },
      });
      return;
    }
    if (input?.exhaustCapabilityStreams === true) {
      let overflowCode = null;
      for (let index = 0; index <= 64; index += 1) {
        const reply = await request("ctx.storage.watch", {
          path: `posts/${index}`,
          streamId: message.params.streamId,
        });
        if (reply.error) {
          overflowCode = reply.error.code;
          break;
        }
      }
      respond(message.id, {
        kind: "single",
        value: { ...input, overflowCode },
      });
      return;
    }
    if (input?.duplicateCapabilityStream === true || input?.cancelCapabilityStream === true
        || input?.staleCapabilityCancel === true || input?.stuckCapabilityCancel === true) {
      const reply = await request("ctx.storage.watch", {
        path: "posts",
        streamId: message.params.streamId,
      });
      const capabilityStreamId = reply.result.streamId;
      let cancelled = null;
      if (input.stuckCapabilityCancel) {
        send({ jsonrpc: "2.0", method: "stream.start", params: { streamId: capabilityStreamId } });
        cancelled = await request("stream.cancel", {
          streamId: message.params.streamId,
          capabilityStreamId,
        });
      } else if (input.cancelCapabilityStream || input.staleCapabilityCancel) {
        cancelled = await request("stream.cancel", {
          streamId: message.params.streamId + (input.staleCapabilityCancel ? 1 : 0),
          capabilityStreamId,
        });
      } else {
        send({ jsonrpc: "2.0", method: "stream.start", params: { streamId: capabilityStreamId } });
      }
      if (input.duplicateCapabilityStream) {
        send({ jsonrpc: "2.0", method: "stream.start", params: { streamId: capabilityStreamId } });
        return;
      }
      respond(message.id, {
        kind: "single",
        value: { ...input, cancelCode: cancelled?.error?.code ?? null },
      });
      return;
    }
    if (input?.invalidLog === true) {
      send({ jsonrpc: "2.0", method: "log", params: { level: "fatal", message: 7 } });
      return;
    }
    if (Number.isSafeInteger(input?.logFlood) && input.logFlood > 0) {
      for (let index = 0; index < input.logFlood; index += 1) {
        send({
          jsonrpc: "2.0",
          method: "log",
          params: { level: "info", message: `flood-${index}`, fields: { index } },
        });
      }
      respond(message.id, { kind: "single", value: input });
      return;
    }
    if (input?.unknownNotification === true) {
      send({ jsonrpc: "2.0", method: "unknown", params: {} });
      return;
    }
    if (streamStage || reduceStage) {
      if (input?.badStreamMeta === true) {
        respond(message.id, { kind: "stream", streamId: message.params.streamId, produced: 1 });
        return;
      }
      if (input?.kind !== "stream-handle") {
        respond(message.id, { kind: "stream", streamId: message.params.streamId, produced: 0 });
        return;
      }
      inputStreams.set(input.streamId, {
        requestId: message.id,
        outputStreamId: message.params.streamId,
        produced: 0,
        reduce: reduceStage,
        values: [],
      });
      return;
    }
    if (input?.readPath) {
      const reply = await request("ctx.storage.read", {
        path: input.readPath,
        streamId: message.params.streamId,
      });
      if (reply.error) {
        send({ jsonrpc: "2.0", id: message.id, error: reply.error });
      } else {
        respond(message.id, {
          kind: "single",
          value: { ...input, bytes: reply.result.bytes },
        });
      }
      return;
    }
    if (input?.probeDenied) {
      const reply = await request("ctx.env.get", {
        name: "FIXTURE_SECRET",
        streamId: message.params.streamId,
      });
      respond(message.id, {
        kind: "single",
        value: { deniedCode: reply.error?.code ?? null },
      });
      return;
    }
    if (input?.probeStale) {
      const reply = await request("ctx.storage.read", {
        path: "stale.md",
        streamId: message.params.streamId + 1,
      });
      respond(message.id, {
        kind: "single",
        value: { staleCode: reply.error?.code ?? null },
      });
      return;
    }
    send({
      jsonrpc: "2.0",
      method: "log",
      params: input?.noLogFields
        ? { level: "info", message: "fixture echo" }
        : { level: "info", message: "fixture echo", fields: { stageId } },
    });
    respond(message.id, { kind: "single", value: input });
    return;
  }

  if (message.method === "stream.value") {
    const state = inputStreams.get(message.params.streamId);
    if (state) {
      state.produced += 1;
      if (state.reduce) state.values.push(message.params.value);
      else {
        send({
          jsonrpc: "2.0",
          method: "stream.value",
          params: { streamId: state.outputStreamId, value: message.params.value },
        });
      }
    }
    return;
  }

  if (message.method === "stream.end") {
    const state = inputStreams.get(message.params.streamId);
    if (state) {
      inputStreams.delete(message.params.streamId);
      if (state.reduce) {
        respond(state.requestId, {
          kind: "single",
          value: { count: state.produced, values: state.values },
        });
      } else {
        respond(state.requestId, {
          kind: "stream",
          streamId: state.outputStreamId,
          produced: state.produced,
        });
      }
    }
    return;
  }

  if (message.method === "stage.dispose") {
    if (ignoreDispose) return;
    respond(message.id, null);
    setTimeout(() => process.exit(0), 5);
  }
  // Deliberately ignore cancellation so the host's kill escalation is tested.
}

process.stdin.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  while (true) {
    const headerEnd = buffer.indexOf("\r\n\r\n");
    if (headerEnd < 0) return;
    const header = buffer.subarray(0, headerEnd).toString("ascii");
    const match = /^Content-Length: ([0-9]+)$/i.exec(header);
    if (!match) process.exit(90);
    const length = Number(match[1]);
    const frameEnd = headerEnd + 4 + length;
    if (buffer.length < frameEnd) return;
    const payload = buffer.subarray(headerEnd + 4, frameEnd);
    buffer = buffer.subarray(frameEnd);
    void handle(decode(JSON.parse(payload.toString("utf8"))));
  }
});
