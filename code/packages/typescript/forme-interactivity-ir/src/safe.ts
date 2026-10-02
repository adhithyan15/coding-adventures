import { types as utilTypes } from "node:util";

import { InteractivityError } from "./error.js";

const SIMPLE_PATH_KEY = /^[A-Za-z_][A-Za-z0-9_-]{0,63}$/;
const MAX_PATH_KEY_SCALARS = 32;

/** Reject Proxy meta-traps before any reflection or property access. */
export function rejectProxy(value: object, path: string): void {
  if (utilTypes.isProxy(value)) {
    throw new InteractivityError("INVALID_TYPE", path, "Proxy values are not allowed");
  }
}

/** Append an attacker-controlled property key without log/control injection. */
export function appendPath(path: string, key: string): string {
  if (SIMPLE_PATH_KEY.test(key)) return `${path}.${key}`;
  let encoded = "";
  let count = 0;
  for (const scalar of key) {
    if (count === MAX_PATH_KEY_SCALARS) {
      encoded += "\\u2026";
      break;
    }
    const point = scalar.codePointAt(0)!;
    if (
      (point >= 0x30 && point <= 0x39)
      || (point >= 0x41 && point <= 0x5a)
      || (point >= 0x61 && point <= 0x7a)
      || point === 0x2d
      || point === 0x5f
    ) {
      encoded += scalar;
    } else if (point <= 0xffff) {
      encoded += `\\u${point.toString(16).padStart(4, "0")}`;
    } else {
      encoded += `\\u{${point.toString(16)}}`;
    }
    count++;
  }
  return `${path}["${encoded}"]`;
}

/** Reject strings that cannot be ordered as Unicode scalar sequences. */
export function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) return false;
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) return false;
      index++;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      return false;
    }
  }
  return true;
}
