/** Stable, bounded errors which are safe for a product shell to display. */

export type AuthoringErrorCode =
  | "INVALID_LIMIT"
  | "INVALID_PROJECT"
  | "INVALID_STATE"
  | "INVALID_COMMAND"
  | "MISSING_INITIAL_PROJECT"
  | "DOCUMENT_NOT_FOUND"
  | "NO_UNDO"
  | "NO_REDO"
  | "STORAGE_CONFLICT"
  | "STORAGE_ERROR"
  | "STORAGE_INDETERMINATE";

export class AuthoringError extends Error {
  readonly code: AuthoringErrorCode;

  constructor(code: AuthoringErrorCode, message: string) {
    super(message);
    this.name = "AuthoringError";
    this.code = code;
  }
}

export function invalidProject(path: string, reason: string): never {
  // Paths are assembled only from schema-owned field names and numeric array
  // indexes. Never interpolate hostile persisted values into this diagnostic.
  throw new AuthoringError("INVALID_PROJECT", `Invalid authoring project at ${path}: ${reason}.`);
}

export function invalidState(reason: string): never {
  throw new AuthoringError("INVALID_STATE", `Invalid stored authoring state: ${reason}.`);
}
