export class MiniSqliteError extends globalThis.Error {
  constructor(message: string) {
    super(message);
    this.name = new.target.name;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

export class Warning extends MiniSqliteError {}

export class Error extends MiniSqliteError {}

export class InterfaceError extends Error {}

export class DatabaseError extends Error {}

export class DataError extends DatabaseError {}

export class OperationalError extends DatabaseError {}

export class IntegrityError extends DatabaseError {}

export class InternalError extends DatabaseError {}

export class ProgrammingError extends DatabaseError {}

export class NotSupportedError extends DatabaseError {}

export function translateError(error: unknown): Error {
  if (error instanceof Error) return error;
  if (error instanceof globalThis.Error) {
    const name = error.name.toLowerCase();
    if (name.includes("table") || error.message.toLowerCase().includes("table")) {
      return new OperationalError(error.message);
    }
    if (name.includes("column") || error.message.toLowerCase().includes("column")) {
      return new OperationalError(error.message);
    }
    return new ProgrammingError(error.message);
  }
  return new InternalError(String(error));
}
