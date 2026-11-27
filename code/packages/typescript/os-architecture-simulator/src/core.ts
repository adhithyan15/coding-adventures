/** JSON values are the portable vocabulary shared by stimuli, state, and effects. */
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export type JsonRecord = { [key: string]: JsonValue };

// These per-value limits keep recursive validation bounded on deeply nested or
// very wide inputs. They are independent of a run budget, which counts steps.
const MAX_JSON_DEPTH = 64;
const MAX_JSON_NODES = 100_000;
const MAX_JSON_STRING_LENGTH = 1_000_000;
const MAX_JSON_TOTAL_TEXT_CODE_UNITS = 10_000_000;

export interface Stimulus {
  kind: string;
  data: JsonRecord;
}

export interface Effect {
  kind: string;
  data: JsonRecord;
}

export interface Transition {
  state: JsonValue;
  effects: Effect[];
}

/** Profiles define all guest policy; the engine only sequences their reducer. */
export interface Profile {
  readonly id: string;
  initialState(): JsonValue;
  transition(state: JsonValue, tick: number, stimulus: Stimulus): Transition;
}

export interface TraceEvent {
  tick: number;
  profile: string;
  kind: string;
  data: JsonRecord;
}

export interface Snapshot {
  schemaVersion: 1;
  profile: string;
  tick: number;
  state: JsonValue;
  pending: Stimulus[];
  history: Stimulus[];
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/** Sort already-normalized object keys recursively for stable JSON output. */
function canonical(value: JsonValue): JsonValue {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === "object") {
    const sorted = Object.create(null) as JsonRecord;
    for (const key of Object.keys(value).sort()) sorted[key] = canonical(value[key]!);
    return sorted;
  }
  return value;
}

interface ValidationBudget { nodes: number; textCodeUnits: number }

/**
 * Copy JSON data into fresh, canonical containers without invoking accessors
 * or `toJSON`. The machine is deterministic, not a sandbox for Proxy traps or
 * profile code; normal JSON objects and arrays are the supported input shape.
 */
function normalizeJson(value: unknown, label: string, depth = 0, budget: ValidationBudget = { nodes: 0, textCodeUnits: 0 }): JsonValue {
  if (depth > MAX_JSON_DEPTH) throw new Error(`${label} exceeds maximum JSON depth (${MAX_JSON_DEPTH})`);
  budget.nodes++;
  if (budget.nodes > MAX_JSON_NODES) throw new Error(`${label} exceeds maximum JSON node count (${MAX_JSON_NODES})`);

  if (value === null || typeof value === "boolean") return value;
  if (typeof value === "string") {
    accountText(value, budget, label);
    return value;
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error(`${label} numbers must be finite`);
    return Object.is(value, -0) ? 0 : value;
  }
  if (typeof value !== "object" || value === null) throw new Error(`${label} must contain only JSON values`);

  const prototype = Object.getPrototypeOf(value);
  if (Array.isArray(value)) {
    if (prototype !== Array.prototype) throw new Error(`${label} arrays must be plain arrays`);
    const length = value.length;
    if (length > MAX_JSON_NODES) throw new Error(`${label} exceeds maximum JSON node count (${MAX_JSON_NODES})`);
    for (const key of Reflect.ownKeys(value)) {
      if (typeof key === "symbol") throw new Error(`${label} cannot contain symbol keys`);
      if (key === "length") continue;
      const index = Number(key);
      if (!Number.isSafeInteger(index) || index < 0 || String(index) !== key || index >= length) {
        throw new Error(`${label} arrays cannot contain extra properties`);
      }
    }
    const result: JsonValue[] = [];
    for (let index = 0; index < length; index++) {
      const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
      if (!descriptor || !descriptor.enumerable || !("value" in descriptor)) {
        throw new Error(`${label} arrays must be dense data arrays without accessors`);
      }
      result.push(normalizeJson(descriptor.value, label, depth + 1, budget));
    }
    return result;
  }

  if (prototype !== Object.prototype && prototype !== null) {
    throw new Error(`${label} records must be plain JSON objects`);
  }
  const keys = Reflect.ownKeys(value);
  if (keys.length > MAX_JSON_NODES) throw new Error(`${label} exceeds maximum JSON node count (${MAX_JSON_NODES})`);
  if (keys.some(key => typeof key === "symbol")) throw new Error(`${label} records cannot contain symbol keys`);
  const stringKeys = keys as string[];
  for (const key of stringKeys) {
    accountText(key, budget, `${label} key`);
  }
  const result = Object.create(null) as JsonRecord;
  for (const key of stringKeys.sort()) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    if (!descriptor || !descriptor.enumerable || !("value" in descriptor)) {
      throw new Error(`${label} records must contain enumerable data properties without accessors`);
    }
    result[key] = normalizeJson(descriptor.value, label, depth + 1, budget);
  }
  return result;
}

function accountText(value: string, budget: ValidationBudget, label: string): void {
  if (value.length > MAX_JSON_STRING_LENGTH) throw new Error(`${label} exceeds maximum length (${MAX_JSON_STRING_LENGTH})`);
  budget.textCodeUnits += value.length;
  if (budget.textCodeUnits > MAX_JSON_TOTAL_TEXT_CODE_UNITS) {
    throw new Error(`${label} exceeds maximum JSON text (${MAX_JSON_TOTAL_TEXT_CODE_UNITS} code units)`);
  }
}

function normalizeRecord(value: unknown, label: string): JsonRecord {
  const normalized = normalizeJson(value, label);
  if (normalized === null || typeof normalized !== "object" || Array.isArray(normalized)) {
    throw new Error(`${label} must be a JSON record`);
  }
  return normalized;
}

function normalizeStimulus(input: Stimulus): Stimulus {
  const record = normalizeRecord(input, "stimulus");
  if (typeof record.kind !== "string" || record.kind.length === 0) throw new Error("stimulus kind is required");
  if (record.data === null || typeof record.data !== "object" || Array.isArray(record.data)) {
    throw new Error("stimulus data must be a JSON record");
  }
  return { kind: record.kind, data: record.data as JsonRecord };
}

function snapshotState(value: unknown): JsonValue {
  return normalizeJson(value, "profile state");
}

/** A bounded, deterministic reducer machine with no OS-specific transitions. */
export class Machine {
  readonly profile: Profile;
  private currentTick = 0;
  private currentState: JsonValue;
  private pending: Stimulus[] = [];
  private processed: Stimulus[] = [];
  private trace: TraceEvent[] = [];

  constructor(profile: Profile) {
    if (!profile.id) throw new Error("profile ID is required");
    this.profile = profile;
    this.currentState = snapshotState(profile.initialState());
  }

  get tick(): number { return this.currentTick; }
  get state(): JsonValue { return clone(this.currentState); }
  get events(): TraceEvent[] { return clone(this.trace); }

  /** Queue external input; it is consumed before automatically generated ticks. */
  inject(stimulus: Stimulus): void {
    this.pending.push(normalizeStimulus(stimulus));
  }

  /** Zero is a no-op; negative and fractional budgets are rejected. */
  run(budget: number): TraceEvent[] {
    if (!Number.isSafeInteger(budget) || budget < 0) throw new Error("run budget must be a nonnegative safe integer");
    const start = this.trace.length;
    for (let index = 0; index < budget; index++) this.transitionOnce();
    return this.trace.slice(start).map(clone);
  }

  /** Process one transition; like run(1), an empty queue supplies clock.tick. */
  step(): TraceEvent[] {
    const start = this.trace.length;
    this.transitionOnce();
    return this.trace.slice(start).map(clone);
  }

  snapshot(): Snapshot {
    return {
      schemaVersion: 1,
      profile: this.profile.id,
      tick: this.currentTick,
      state: clone(this.currentState),
      pending: clone(this.pending),
      history: clone(this.processed),
    };
  }

  serialize(): string {
    return JSON.stringify(canonical(this.snapshot() as unknown as JsonValue));
  }

  static restore(profile: Profile, encoded: string): Machine {
    const parsed: unknown = JSON.parse(encoded);
    if (parsed === null || typeof parsed !== "object") throw new Error("snapshot must be an object");
    const snapshot = parsed as Partial<Snapshot>;
    if (snapshot.schemaVersion !== 1 || snapshot.profile !== profile.id) throw new Error("snapshot schema or profile mismatch");
    if (!Number.isSafeInteger(snapshot.tick) || !Array.isArray(snapshot.history) || !Array.isArray(snapshot.pending)) throw new Error("snapshot fields are invalid");
    const machine = new Machine(profile);
    for (const stimulus of snapshot.history) machine.transitionOnce(normalizeStimulus(stimulus));
    if (machine.tick !== snapshot.tick || JSON.stringify(machine.currentState) !== JSON.stringify(snapshotState(snapshot.state))) {
      throw new Error("snapshot state does not match replayed history");
    }
    machine.pending = snapshot.pending.map(normalizeStimulus);
    return machine;
  }

  static replay(profile: Profile, history: Stimulus[]): Machine {
    const machine = new Machine(profile);
    for (const stimulus of history) machine.transitionOnce(normalizeStimulus(stimulus));
    return machine;
  }

  private transitionOnce(explicit?: Stimulus): void {
    if (this.currentTick >= Number.MAX_SAFE_INTEGER) throw new Error("logical tick limit reached");
    const stimulus = explicit ?? this.pending[0] ?? { kind: "clock.tick", data: {} };
    const nextTick = this.currentTick + 1;
    // Work on isolated JSON copies. A thrown reducer or invalid effect leaves
    // tick, state, queue, history, and trace unchanged.
    const transition = this.profile.transition(clone(this.currentState), nextTick, clone(stimulus));
    const normalizedTransition = normalizeRecord(transition, "profile transition");
    const nextState = snapshotState(normalizedTransition.state);
    const effects = normalizedTransition.effects;
    if (!Array.isArray(effects)) throw new Error("profile transition effects must be an array");
    const additions = effects.map((value): TraceEvent => {
      if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("profile effect must be a record");
      const effect = value as JsonRecord;
      if (typeof effect.kind !== "string" || effect.kind.length === 0) throw new Error("effect kind is required");
      if (effect.data === null || typeof effect.data !== "object" || Array.isArray(effect.data)) throw new Error("effect data must be a JSON record");
      return { tick: nextTick, profile: this.profile.id, kind: effect.kind, data: effect.data as JsonRecord };
    });
    if (explicit === undefined && this.pending.length > 0) this.pending.shift();
    this.currentTick = nextTick;
    this.currentState = nextState;
    this.processed.push(clone(stimulus));
    this.trace.push(...additions);
  }
}
