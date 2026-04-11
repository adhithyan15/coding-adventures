import {
  array,
  bulkString,
  type RespArray,
  type RespBulkString,
  type RespValue,
} from "@coding-adventures/resp-protocol";

export interface DataStoreCommand {
  readonly name: string;
  readonly args: string[];
}

export function commandName(parts: string[]): string {
  return commandFromParts(parts).name;
}

export function commandFromParts(parts: string[]): DataStoreCommand {
  if (parts.length === 0) {
    throw new Error("command frame cannot be empty");
  }
  return {
    name: parts[0].trim().toUpperCase(),
    args: parts.slice(1),
  };
}

export function commandToParts(command: DataStoreCommand): string[] {
  return [command.name, ...command.args];
}

export function commandFromResp(value: RespValue): DataStoreCommand | null {
  if (value.kind !== "array" || value.value === null || value.value.length === 0) {
    return null;
  }

  const parts: string[] = [];
  for (const element of value.value) {
    const part = respValueToString(element);
    if (part === null) {
      return null;
    }
    parts.push(part);
  }
  return commandFromParts(parts);
}

export function commandToResp(command: DataStoreCommand): RespArray {
  return array(commandToParts(command).map((part) => bulkString(part)));
}

export function respValueToString(value: RespValue): string | null {
  switch (value.kind) {
    case "simple-string":
    case "error":
      return value.value;
    case "integer":
      return String(value.value);
    case "bulk-string":
      return value.value === null ? null : new TextDecoder().decode(value.value);
    case "array":
      return null;
  }
}

export function commandToRespValue(command: DataStoreCommand): RespValue {
  return commandToResp(command);
}

export function commandFrameToResp(parts: string[]): RespArray {
  return array(parts.map((part) => bulkString(part)));
}
