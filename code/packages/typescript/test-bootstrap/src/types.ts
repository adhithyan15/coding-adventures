/**
 * TF01 types for values, records, and test verdicts.
 */

export type Value =
  | { kind: "null" }
  | { kind: "bool"; value: boolean }
  | { kind: "int"; text: string }
  | { kind: "f64"; bits: bigint }
  | { kind: "bytes"; value: Uint8Array }
  | { kind: "text"; value: string }
  | { kind: "list"; items: Value[] }
  | { kind: "map"; entries: [Value, Value][] };

export interface Record {
  id: string;
  fields: Map<string, Value>;
}

export type Verdict =
  | { status: "pass" }
  | { status: "fail"; failureId: string; message: string };

export type FailureId =
  | "not-deep-equal"
  | "no-error"
  | "wrong-error"
  | "panic"
  | "corpus-mismatch"
  | "corpus-malformed"
  | "property-counterexample"
  | "property-exhausted"
  | "property-timeout"
  | "property-panic"
  | "fuzz-crash"
  | "fuzz-timeout"
  | "fuzz-oom"
  | "fuzz-leak"
  | "fuzz-panic";

export const REGISTERED_FAILURE_IDS: ReadonlySet<string> = new Set<string>([
  "not-deep-equal",
  "no-error",
  "wrong-error",
  "panic",
  "corpus-mismatch",
  "corpus-malformed",
  "property-counterexample",
  "property-exhausted",
  "property-timeout",
  "property-panic",
  "fuzz-crash",
  "fuzz-timeout",
  "fuzz-oom",
  "fuzz-leak",
  "fuzz-panic",
]);
