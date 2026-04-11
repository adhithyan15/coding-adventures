import { DataStoreEngine, type DataStoreModule, type Store, createDataStoreEngine } from "@coding-adventures/in-memory-data-store-engine";
import {
  commandFromResp,
  type DataStoreCommand,
} from "@coding-adventures/in-memory-data-store-protocol";
import {
  RespDecoder,
  type RespArray,
  type RespValue,
  array,
  bulkString,
  encode,
  errorValue,
  simpleString,
} from "@coding-adventures/resp-protocol";

export interface InMemoryDataStoreOptions {
  readonly engine?: DataStoreEngine;
  readonly store?: Store;
}

export class InMemoryDataStore {
  private engine: DataStoreEngine;
  private decoder: RespDecoder;

  constructor(options: InMemoryDataStoreOptions = {}) {
    this.engine = options.engine ?? createDataStoreEngine(options.store);
    this.decoder = new RespDecoder();
  }

  get store(): Store {
    return this.engine.store;
  }

  registerModule(module: DataStoreModule): this {
    this.engine.registerModule(module);
    return this;
  }

  reset(store?: Store): void {
    this.engine.reset(store ?? this.engine.store);
    this.decoder = new RespDecoder();
  }

  execute(command: DataStoreCommand | string[]): RespValue {
    return this.engine.execute(command);
  }

  executeCommand(command: DataStoreCommand): RespValue {
    return this.engine.execute(command);
  }

  executeParts(parts: string[]): RespValue {
    return this.engine.execute(parts);
  }

  executeFrame(frame: RespValue): RespValue | null {
    if (frame.kind !== "array" || frame.value === null) {
      return errorValue("ERR expected RESP array command");
    }
    if (frame.value.length === 0) {
      return null;
    }
    const command = commandFromResp(frame);
    if (command === null) {
      return errorValue("ERR expected RESP command array");
    }
    return this.engine.execute(command);
  }

  process(input: Uint8Array | string): RespValue[] {
    this.decoder.feed(input);
    const responses: RespValue[] = [];
    while (this.decoder.hasMessage()) {
      const frame = this.decoder.getMessage();
      const response = this.executeFrame(frame);
      if (response !== null) {
        responses.push(response);
      }
    }
    return responses;
  }

  handle(input: Uint8Array | string): Uint8Array {
    return encodeRespStream(this.process(input));
  }
}

export function createInMemoryDataStore(options: InMemoryDataStoreOptions = {}): InMemoryDataStore {
  return new InMemoryDataStore(options);
}

export function encodeRespStream(values: RespValue[]): Uint8Array {
  return concatBytes(values.map((value) => encode(value)));
}

export function concatBytes(chunks: Uint8Array[]): Uint8Array {
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const result = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.length;
  }
  return result;
}

export function commandToFrame(command: DataStoreCommand): RespArray {
  return array(command.args.length === 0 ? [bulkString(command.name)] : [bulkString(command.name), ...command.args.map((arg) => bulkString(arg))]);
}

export function frameToResponseText(frame: RespValue): string {
  if (frame.kind === "simple-string" || frame.kind === "error") {
    return frame.value;
  }
  if (frame.kind === "integer") {
    return String(frame.value);
  }
  if (frame.kind === "bulk-string") {
    return frame.value === null ? "(nil)" : new TextDecoder().decode(frame.value);
  }
  return frame.value === null ? "(nil)" : `[array:${frame.value.length}]`;
}

export function ok(): RespValue {
  return simpleString("OK");
}
