/**
 * Discriminated union representation of values for bit-exact comparison (TF01 §3.1).
 */
export type Value =
  | { kind: "null" }
  | { kind: "undefined" }
  | { kind: "bool"; value: boolean }
  | { kind: "int"; text: string }
  | { kind: "f64"; bits: bigint }
  | { kind: "bytes"; value: Uint8Array }
  | { kind: "text"; value: string }
  | { kind: "date"; ms: number }
  | { kind: "regexp"; source: string; flags: string }
  | { kind: "error"; name: string; message: string }
  | { kind: "list"; items: Value[] }
  | { kind: "map"; entries: [Value, Value][] };
