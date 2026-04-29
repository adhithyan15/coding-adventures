export { bindParameters } from "./binding.js";
export { Connection, connect, type ConnectOptions, type RowTuple } from "./connection.js";
export { Cursor, type DescriptionItem } from "./cursor.js";
export {
  DataError,
  DatabaseError,
  Error,
  IntegrityError,
  InterfaceError,
  InternalError,
  MiniSqliteError,
  NotSupportedError,
  OperationalError,
  ProgrammingError,
  Warning,
} from "./errors.js";

export const apilevel = "2.0";
export const threadsafety = 1;
export const paramstyle = "qmark";
