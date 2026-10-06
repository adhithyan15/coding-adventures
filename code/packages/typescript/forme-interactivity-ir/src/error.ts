import type { InteractivityErrorCode } from "./types.js";

/** Stable fail-closed validation error with a bounded path and message. */
export class InteractivityError extends Error {
  readonly code: InteractivityErrorCode;
  readonly path: string;

  constructor(code: InteractivityErrorCode, path: string, message: string) {
    super(`${code} at ${path}: ${message}`);
    this.name = "InteractivityError";
    this.code = code;
    this.path = path;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}
