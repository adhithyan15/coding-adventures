import type { JsonValue } from "@coding-adventures/forme-types";

import {
  registerValidatedInteractivityDocument,
} from "./canonical.js";
import { InteractivityError } from "./error.js";
import { HARD_INTERACTIVITY_LIMITS, snapshotLimits } from "./limits.js";
import { appendPath, isWellFormedUnicode, rejectProxy } from "./safe.js";
import type {
  Binding,
  BindingApplication,
  Effect,
  Handler,
  InteractivityDocument,
  InteractivityLimits,
  IslandDeclaration,
  NodeRef,
  Predicate,
  StateDeclaration,
  StateType,
  Trigger,
  ValueExpr,
} from "./types.js";

const IDENTIFIER = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/;
const PACKAGE_NAME = /^(?:@[a-z0-9][a-z0-9-]*\/)?[a-z0-9][a-z0-9-]*$/;
const EXPORT_NAME = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;
const SIMPLE_TRIGGERS = new Set(["click", "focus", "blur", "input", "change", "submit"]);
const encoder = new TextEncoder();

type Fields = Readonly<Record<string, unknown>>;

class Validator {
  private readonly seen = new WeakSet<object>();
  private readonly stateByName = new Map<string, StateDeclaration>();
  private readonly islandById = new Map<string, IslandDeclaration>();
  private readonly usedIslands = new Set<string>();
  private readonly descriptors = new WeakMap<object, ReadonlyMap<string, PropertyDescriptor>>();
  private readonly prototypes = new WeakMap<object, object | null>();
  private readonly preflightSeen = new WeakSet<object>();
  private expressionNodes = 0;
  private jsonNodes = 0;
  private canonicalBytes = 0;
  private preflightNodes = 0;

  constructor(private readonly limits: Readonly<InteractivityLimits>) {}

  validate(input: unknown): InteractivityDocument {
    this.preflight(input, "$", 1);
    return this.document(input);
  }

  document(input: unknown): InteractivityDocument {
    const fields = this.object(input, "$", ["kind", "version", "state", "bindings", "handlers", "islands"]);
    this.literal(fields.kind, "Interactivity", "$.kind");
    this.literal(fields.version, 1, "$.version");

    const stateRaw = this.array(fields.state, "$.state", this.limits.maxState);
    const state = stateRaw.map((value, index) => this.state(value, `$.state[${index}]`));
    const islandsRaw = this.array(fields.islands, "$.islands", this.limits.maxIslands);
    const islands = islandsRaw.map((value, index) => this.island(value, `$.islands[${index}]`));
    const bindingsRaw = this.array(fields.bindings, "$.bindings", this.limits.maxBindings);
    const bindings = bindingsRaw.map((value, index) => this.binding(value, `$.bindings[${index}]`));
    const handlersRaw = this.array(fields.handlers, "$.handlers", this.limits.maxHandlers);
    const handlers = handlersRaw.map((value, index) => this.handler(value, `$.handlers[${index}]`));

    for (let index = 0; index < islands.length; index++) {
      const island = islands[index]!;
      if (island.activation === "interaction" && !this.usedIslands.has(island.id)) {
        this.fail("DANGLING_REFERENCE", `$.islands[${index}].id`, "interaction island is not referenced by a handler");
      }
    }

    const result: InteractivityDocument = Object.freeze({
      kind: "Interactivity",
      version: 1,
      state: Object.freeze(state),
      bindings: Object.freeze(bindings),
      handlers: Object.freeze(handlers),
      islands: Object.freeze(islands),
    });
    registerValidatedInteractivityDocument(result);
    return result;
  }

  private preflight(input: unknown, path: string, depth: number): void {
    const maxDepth = this.limits.maxExpressionDepth + this.limits.maxJsonDepth + 16;
    if (depth > maxDepth) this.fail("LIMIT_EXCEEDED", path, "document nesting limit exceeded");
    this.preflightNodes++;
    if (this.preflightNodes > this.limits.maxCanonicalBytes) {
      this.fail("LIMIT_EXCEEDED", "$", "document structural node limit exceeded");
    }

    if (input === null) {
      this.addCanonicalBytes(4);
      return;
    }
    if (typeof input === "boolean") {
      this.addCanonicalBytes(input ? 4 : 5);
      return;
    }
    if (typeof input === "number") {
      if (!Number.isFinite(input)) this.fail("INVALID_VALUE", path, "number must be finite");
      this.addCanonicalBytes(encoder.encode(JSON.stringify(input)).byteLength);
      return;
    }
    if (typeof input === "string") {
      this.preflightString(input, path);
      return;
    }
    if (input === undefined || typeof input === "bigint" || typeof input === "function" || typeof input === "symbol") {
      this.fail("INVALID_TYPE", path, "value is not canonical JSON data");
    }

    rejectProxy(input, path);
    if (this.preflightSeen.has(input)) this.fail("REPEATED_VALUE", path, "repeated or cyclic object identity");
    this.preflightSeen.add(input);
    const isArray = Array.isArray(input);
    const prototype = Object.getPrototypeOf(input);
    this.prototypes.set(input, prototype);
    if (isArray ? prototype !== Array.prototype : prototype !== Object.prototype && prototype !== null) {
      this.fail("INVALID_TYPE", path, isArray ? "array must use Array.prototype" : "plain object required");
    }
    let maximumArrayEntries = 0;
    if (isArray) {
      maximumArrayEntries = Math.max(
        this.limits.maxState,
        this.limits.maxBindings,
        this.limits.maxHandlers,
        this.limits.maxIslands,
        this.limits.maxEffectsPerHandler,
        this.limits.maxPredicateOperands,
        this.limits.maxJsonNodes,
        256,
      );
      const remainingBytes = this.limits.maxCanonicalBytes - this.canonicalBytes;
      if (input.length > maximumArrayEntries || input.length * 2 + 1 > remainingBytes) {
        this.fail("LIMIT_EXCEEDED", path, "array cannot fit within document limits");
      }
    }

    const ownKeys = Reflect.ownKeys(input);
    const maximumOwnKeys = isArray
      ? Math.min(maximumArrayEntries + 1, input.length + 65)
      : Math.min(
        HARD_INTERACTIVITY_LIMITS.maxJsonNodes,
        Math.floor((this.limits.maxCanonicalBytes - this.canonicalBytes) / 4),
      );
    if (ownKeys.length > maximumOwnKeys) this.fail("LIMIT_EXCEEDED", path, "object property limit exceeded");
    if (ownKeys.some(key => typeof key === "symbol")) this.fail("SYMBOL_KEY", path, "symbol keys are not allowed");
    const keys = ownKeys as string[];
    const descriptors = new Map<string, PropertyDescriptor>();
    for (const key of keys) {
      const keyPath = appendPath(path, key);
      if (!isArray) this.preflightString(key, keyPath);
      const descriptor = Object.getOwnPropertyDescriptor(input, key);
      if (descriptor === undefined) throw new Error("internal error: own key has no descriptor");
      descriptors.set(key, descriptor);
    }
    this.descriptors.set(input, descriptors);

    if (isArray) {
      this.addCanonicalBytes(2);
      for (let index = 0; index < input.length; index++) {
        const descriptor = descriptors.get(String(index));
        if (descriptor === undefined) this.fail("SPARSE_ARRAY", `${path}[${index}]`, "array holes are not allowed");
        if (!("value" in descriptor)) this.fail("ACCESSOR", `${path}[${index}]`, "array accessors are not allowed");
        if (!descriptor.enumerable) this.fail("INVALID_VALUE", `${path}[${index}]`, "array entries must be enumerable");
        if (index > 0) this.addCanonicalBytes(1);
        this.preflight(descriptor.value, `${path}[${index}]`, depth + 1);
      }
      for (const key of descriptors.keys()) {
        if (key !== "length" && !/^(?:0|[1-9][0-9]*)$/.test(key)) {
          this.fail("UNKNOWN_FIELD", appendPath(path, key), "array properties are not allowed");
        }
      }
      return;
    }

    this.addCanonicalBytes(2);
    for (let index = 0; index < keys.length; index++) {
      const key = keys[index]!;
      const keyPath = appendPath(path, key);
      const descriptor = descriptors.get(key)!;
      if (!("value" in descriptor)) this.fail("ACCESSOR", keyPath, "accessors are not allowed");
      if (!descriptor.enumerable) this.fail("INVALID_VALUE", keyPath, "fields must be enumerable");
      if (index > 0) this.addCanonicalBytes(1);
      this.addCanonicalBytes(1);
      this.preflight(descriptor.value, keyPath, depth + 1);
    }
  }

  private preflightString(value: string, path: string): void {
    if (value.length > this.limits.maxStringBytes) this.fail("LIMIT_EXCEEDED", path, "string byte limit exceeded");
    if (!isWellFormedUnicode(value)) this.fail("INVALID_VALUE", path, "string must contain well-formed Unicode scalar values");
    if (encoder.encode(value).byteLength > this.limits.maxStringBytes) this.fail("LIMIT_EXCEEDED", path, "string byte limit exceeded");
    this.addCanonicalBytes(encoder.encode(JSON.stringify(value)).byteLength);
  }

  private addCanonicalBytes(count: number): void {
    this.canonicalBytes += count;
    if (this.canonicalBytes > this.limits.maxCanonicalBytes) {
      this.fail("LIMIT_EXCEEDED", "$", "canonical document exceeds maxCanonicalBytes");
    }
  }

  private state(input: unknown, path: string): StateDeclaration {
    const base = this.object(input, path, ["name", "scope", "type", "initial", "persist"], ["owner", "values"]);
    const name = this.identifier(base.name, `${path}.name`);
    if (this.stateByName.has(name)) this.fail("DUPLICATE_ID", `${path}.name`, "duplicate state name");
    const scope = this.oneOf(base.scope, ["block", "document", "site", "session"] as const, `${path}.scope`);
    const type = this.oneOf(base.type, ["boolean", "number", "string", "enum", "json"] as const, `${path}.type`);
    const persist = this.oneOf(base.persist, ["none", "session", "local"] as const, `${path}.persist`);

    let owner: NodeRef | undefined;
    if (scope === "block") {
      if (!("owner" in base)) this.fail("MISSING_FIELD", `${path}.owner`, "block state requires owner");
      owner = this.nodeRef(base.owner, `${path}.owner`, true);
    } else if ("owner" in base) {
      this.fail("UNKNOWN_FIELD", `${path}.owner`, "owner is allowed only for block state");
    }

    let values: readonly string[] | undefined;
    if (type === "enum") {
      if (!("values" in base)) this.fail("MISSING_FIELD", `${path}.values`, "enum state requires values");
      const rawValues = this.array(base.values, `${path}.values`, 256);
      if (rawValues.length === 0) this.fail("INVALID_VALUE", `${path}.values`, "enum values must not be empty");
      const unique = new Set<string>();
      values = Object.freeze(rawValues.map((value, index) => {
        const item = this.string(value, `${path}.values[${index}]`);
        if (unique.has(item)) this.fail("DUPLICATE_ID", `${path}.values[${index}]`, "duplicate enum value");
        unique.add(item);
        return item;
      }));
    } else if ("values" in base) {
      this.fail("UNKNOWN_FIELD", `${path}.values`, "values is allowed only for enum state");
    }

    const initial = this.json(base.initial, `${path}.initial`, 1);
    if (!this.valueMatches(type, initial, values)) {
      this.fail("TYPE_MISMATCH", `${path}.initial`, "initial value does not match declared state type");
    }
    const result = Object.freeze({
      name,
      scope,
      type,
      initial,
      persist,
      ...(owner === undefined ? {} : { owner }),
      ...(values === undefined ? {} : { values }),
    }) as StateDeclaration;
    this.stateByName.set(name, result);
    return result;
  }

  private island(input: unknown, path: string): IslandDeclaration {
    const fields = this.object(input, path, ["id", "packageName", "export", "target", "fallback", "activation", "config"]);
    const id = this.identifier(fields.id, `${path}.id`);
    if (this.islandById.has(id)) this.fail("DUPLICATE_ID", `${path}.id`, "duplicate island id");
    const packageName = this.string(fields.packageName, `${path}.packageName`);
    if (packageName.length > 214 || !PACKAGE_NAME.test(packageName)) {
      this.fail("INVALID_IDENTIFIER", `${path}.packageName`, "invalid package name");
    }
    const exportName = this.string(fields.export, `${path}.export`);
    if (!EXPORT_NAME.test(exportName)) this.fail("INVALID_IDENTIFIER", `${path}.export`, "invalid export name");
    const result: IslandDeclaration = Object.freeze({
      id,
      packageName,
      export: exportName,
      target: this.nodeRef(fields.target, `${path}.target`, true) as Extract<NodeRef, { kind: "element" }>,
      fallback: this.nodeRef(fields.fallback, `${path}.fallback`, true) as Extract<NodeRef, { kind: "element" }>,
      activation: this.oneOf(fields.activation, ["load", "visible", "interaction"] as const, `${path}.activation`),
      config: this.json(fields.config, `${path}.config`, 1),
    });
    this.islandById.set(id, result);
    return result;
  }

  private binding(input: unknown, path: string): Binding {
    const fields = this.object(input, path, ["id", "target", "when", "apply"]);
    const id = this.identifier(fields.id, `${path}.id`);
    const bindings = this.bindingIds;
    if (bindings.has(id)) this.fail("DUPLICATE_ID", `${path}.id`, "duplicate binding id");
    bindings.add(id);
    return Object.freeze({
      id,
      target: this.nodeRef(fields.target, `${path}.target`),
      when: this.predicate(fields.when, `${path}.when`, 1),
      apply: this.bindingApplication(fields.apply, `${path}.apply`),
    });
  }

  private readonly bindingIds = new Set<string>();
  private readonly handlerIds = new Set<string>();

  private bindingApplication(input: unknown, path: string): BindingApplication {
    const kind = this.peekKind(input, path);
    if (kind === "visible") {
      const fields = this.object(input, path, ["kind", "whenTrue"]);
      return Object.freeze({ kind, whenTrue: this.boolean(fields.whenTrue, `${path}.whenTrue`) });
    }
    if (kind === "text" || kind === "value") {
      const fields = this.object(input, path, ["kind", "value"]);
      return Object.freeze({ kind, value: this.valueExpr(fields.value, `${path}.value`, 1) });
    }
    if (kind === "extension") {
      const fields = this.object(input, path, ["kind", "name", "value"]);
      return Object.freeze({
        kind,
        name: this.identifier(fields.name, `${path}.name`),
        value: this.json(fields.value, `${path}.value`, 1),
      });
    }
    this.fail("INVALID_VALUE", `${path}.kind`, "unknown binding application kind");
  }

  private handler(input: unknown, path: string): Handler {
    const fields = this.object(input, path, ["id", "target", "on", "effects"]);
    const id = this.identifier(fields.id, `${path}.id`);
    if (this.handlerIds.has(id)) this.fail("DUPLICATE_ID", `${path}.id`, "duplicate handler id");
    this.handlerIds.add(id);
    const rawEffects = this.array(fields.effects, `${path}.effects`, this.limits.maxEffectsPerHandler);
    if (rawEffects.length === 0) this.fail("INVALID_VALUE", `${path}.effects`, "handler must contain an effect");
    return Object.freeze({
      id,
      target: this.nodeRef(fields.target, `${path}.target`),
      on: this.trigger(fields.on, `${path}.on`),
      effects: Object.freeze(rawEffects.map((value, index) => this.effect(value, `${path}.effects[${index}]`))),
    });
  }

  private trigger(input: unknown, path: string): Trigger {
    const kind = this.peekKind(input, path);
    if (SIMPLE_TRIGGERS.has(kind)) {
      this.object(input, path, ["kind"]);
      return Object.freeze({ kind }) as Trigger;
    }
    if (kind === "visible") {
      const fields = this.object(input, path, ["kind", "threshold"]);
      const threshold = this.number(fields.threshold, `${path}.threshold`);
      if (threshold < 0 || threshold > 1) this.fail("INVALID_VALUE", `${path}.threshold`, "threshold must be between 0 and 1");
      return Object.freeze({ kind, threshold });
    }
    if (kind === "timer") {
      const fields = this.object(input, path, ["kind", "afterMs"]);
      const afterMs = this.number(fields.afterMs, `${path}.afterMs`);
      if (!Number.isSafeInteger(afterMs) || afterMs < 0 || afterMs > 86_400_000) {
        this.fail("INVALID_VALUE", `${path}.afterMs`, "timer must be a safe integer between 0 and 86400000");
      }
      return Object.freeze({ kind, afterMs });
    }
    if (kind === "custom") {
      const fields = this.object(input, path, ["kind", "name"]);
      return Object.freeze({ kind, name: this.identifier(fields.name, `${path}.name`) });
    }
    this.fail("INVALID_VALUE", `${path}.kind`, "unknown trigger kind");
  }

  private effect(input: unknown, path: string): Effect {
    const kind = this.peekKind(input, path);
    if (kind === "set-state") {
      const fields = this.object(input, path, ["kind", "state", "value"]);
      const stateName = this.identifier(fields.state, `${path}.state`);
      const declaration = this.resolveState(stateName, `${path}.state`);
      const value = this.valueExpr(fields.value, `${path}.value`, 1);
      if (!this.expressionMatches(declaration, value)) this.fail("TYPE_MISMATCH", `${path}.value`, "effect value is incompatible with state");
      return Object.freeze({ kind, state: stateName, value });
    }
    if (kind === "toggle-state") {
      const fields = this.object(input, path, ["kind", "state"]);
      const stateName = this.identifier(fields.state, `${path}.state`);
      if (this.resolveState(stateName, `${path}.state`).type !== "boolean") {
        this.fail("TYPE_MISMATCH", `${path}.state`, "toggle-state requires boolean state");
      }
      return Object.freeze({ kind, state: stateName });
    }
    if (kind === "navigate") {
      const fields = this.object(input, path, ["kind", "to"]);
      const to = this.string(fields.to, `${path}.to`);
      if (!this.safeNavigation(to)) this.fail("INVALID_URL", `${path}.to`, "unsafe or non-portable navigation target");
      return Object.freeze({ kind, to });
    }
    if (kind === "dispatch") {
      const fields = this.object(input, path, ["kind", "event", "detail"]);
      return Object.freeze({
        kind,
        event: this.identifier(fields.event, `${path}.event`),
        detail: this.json(fields.detail, `${path}.detail`, 1),
      });
    }
    if (kind === "run-island") {
      const fields = this.object(input, path, ["kind", "island", "args"]);
      const island = this.identifier(fields.island, `${path}.island`);
      if (!this.islandById.has(island)) this.fail("DANGLING_REFERENCE", `${path}.island`, "unknown island");
      this.usedIslands.add(island);
      return Object.freeze({ kind, island, args: this.json(fields.args, `${path}.args`, 1) });
    }
    this.fail("INVALID_VALUE", `${path}.kind`, "unknown effect kind");
  }

  private predicate(input: unknown, path: string, depth: number): Predicate {
    this.expressionNode(path, depth);
    const kind = this.peekKind(input, path);
    if (kind === "truthy") {
      const fields = this.object(input, path, ["kind", "value"]);
      return Object.freeze({ kind, value: this.valueExpr(fields.value, `${path}.value`, depth + 1) });
    }
    if (kind === "equals") {
      const fields = this.object(input, path, ["kind", "left", "right"]);
      return Object.freeze({
        kind,
        left: this.valueExpr(fields.left, `${path}.left`, depth + 1),
        right: this.valueExpr(fields.right, `${path}.right`, depth + 1),
      });
    }
    if (kind === "not") {
      const fields = this.object(input, path, ["kind", "predicate"]);
      return Object.freeze({ kind, predicate: this.predicate(fields.predicate, `${path}.predicate`, depth + 1) });
    }
    if (kind === "all" || kind === "any") {
      const fields = this.object(input, path, ["kind", "predicates"]);
      const raw = this.array(fields.predicates, `${path}.predicates`, this.limits.maxPredicateOperands);
      if (raw.length === 0) this.fail("INVALID_VALUE", `${path}.predicates`, "predicate list must not be empty");
      return Object.freeze({
        kind,
        predicates: Object.freeze(raw.map((value, index) => this.predicate(value, `${path}.predicates[${index}]`, depth + 1))),
      });
    }
    this.fail("INVALID_VALUE", `${path}.kind`, "unknown predicate kind");
  }

  private valueExpr(input: unknown, path: string, depth: number): ValueExpr {
    this.expressionNode(path, depth);
    const kind = this.peekKind(input, path);
    if (kind === "literal") {
      const fields = this.object(input, path, ["kind", "value"]);
      return Object.freeze({ kind, value: this.json(fields.value, `${path}.value`, 1) });
    }
    if (kind === "state") {
      const fields = this.object(input, path, ["kind", "state"]);
      const state = this.identifier(fields.state, `${path}.state`);
      this.resolveState(state, `${path}.state`);
      return Object.freeze({ kind, state });
    }
    this.fail("INVALID_VALUE", `${path}.kind`, "unknown value expression kind");
  }

  private expressionNode(path: string, depth: number): void {
    if (depth > this.limits.maxExpressionDepth) this.fail("LIMIT_EXCEEDED", path, "expression depth exceeded");
    this.expressionNodes++;
    if (this.expressionNodes > this.limits.maxExpressionNodes) this.fail("LIMIT_EXCEEDED", path, "expression node limit exceeded");
  }

  private nodeRef(input: unknown, path: string, elementOnly = false): NodeRef {
    const kind = this.peekKind(input, path);
    if (kind === "document") {
      this.object(input, path, ["kind"]);
      if (elementOnly) this.fail("INVALID_VALUE", `${path}.kind`, "element reference required");
      return Object.freeze({ kind });
    }
    if (kind === "element") {
      const fields = this.object(input, path, ["kind", "id"]);
      return Object.freeze({ kind, id: this.identifier(fields.id, `${path}.id`) });
    }
    this.fail("INVALID_VALUE", `${path}.kind`, "unknown node reference kind");
  }

  private json(input: unknown, path: string, depth: number): JsonValue {
    if (depth > this.limits.maxJsonDepth) this.fail("LIMIT_EXCEEDED", path, "JSON depth exceeded");
    this.jsonNodes++;
    if (this.jsonNodes > this.limits.maxJsonNodes) this.fail("LIMIT_EXCEEDED", path, "JSON node limit exceeded");
    if (input === null || typeof input === "boolean") return input;
    if (typeof input === "string") return this.string(input, path);
    if (typeof input === "number") return this.number(input, path);
    if (Array.isArray(input)) {
      const raw = this.array(input, path, this.limits.maxJsonNodes);
      return Object.freeze(raw.map((value, index) => this.json(value, `${path}[${index}]`, depth + 1)));
    }
    if (typeof input === "object") {
      const fields = this.jsonObject(input, path);
      const result: Record<string, JsonValue> = Object.create(null) as Record<string, JsonValue>;
      for (const key of Object.keys(fields)) {
        const keyPath = appendPath(path, key);
        this.string(key, keyPath);
        result[key] = this.json(fields[key], keyPath, depth + 1);
      }
      return Object.freeze(result);
    }
    this.fail("INVALID_TYPE", path, "value is not JSON");
  }

  private jsonObject(input: object, path: string): Fields {
    const prototype = this.prototypeOf(input);
    if (prototype !== Object.prototype && prototype !== null) {
      this.fail("INVALID_TYPE", path, "JSON object must be plain");
    }
    this.mark(input, path);
    const descriptors = this.describe(input);
    const result: Record<string, unknown> = Object.create(null) as Record<string, unknown>;
    for (const [key, descriptor] of descriptors) {
      const keyPath = appendPath(path, key);
      if (!("value" in descriptor)) this.fail("ACCESSOR", keyPath, "accessors are not allowed");
      if (!descriptor.enumerable) this.fail("INVALID_VALUE", keyPath, "fields must be enumerable");
      result[key] = descriptor.value;
    }
    return result;
  }

  private object(input: unknown, path: string, required: readonly string[], optional: readonly string[] = []): Fields {
    if (input === null || typeof input !== "object" || Array.isArray(input)) this.fail("INVALID_TYPE", path, "plain object required");
    const prototype = this.prototypeOf(input);
    if (prototype !== Object.prototype && prototype !== null) {
      this.fail("INVALID_TYPE", path, "plain object required");
    }
    this.mark(input, path);
    const descriptors = this.describe(input);
    const allowed = new Set([...required, ...optional]);
    for (const [key, descriptor] of descriptors) {
      const keyPath = appendPath(path, key);
      if (!("value" in descriptor)) this.fail("ACCESSOR", keyPath, "accessors are not allowed");
      if (!descriptor.enumerable) this.fail("INVALID_VALUE", keyPath, "fields must be enumerable");
      if (!allowed.has(key)) this.fail("UNKNOWN_FIELD", keyPath, "unknown field");
    }
    const result: Record<string, unknown> = {};
    for (const key of required) {
      const descriptor = descriptors.get(key);
      if (descriptor === undefined) this.fail("MISSING_FIELD", `${path}.${key}`, "required field is missing");
      result[key] = descriptor.value;
    }
    for (const key of optional) {
      const descriptor = descriptors.get(key);
      if (descriptor !== undefined) result[key] = descriptor.value;
    }
    return result;
  }

  private array(input: unknown, path: string, maximum: number): readonly unknown[] {
    if (input === null || typeof input !== "object") this.fail("INVALID_TYPE", path, "array required");
    rejectProxy(input, path);
    if (!Array.isArray(input) || this.prototypeOf(input) !== Array.prototype) this.fail("INVALID_TYPE", path, "array required");
    this.mark(input, path);
    if (input.length > maximum) this.fail("LIMIT_EXCEEDED", path, "array length limit exceeded");
    const descriptors = this.describe(input);
    for (let index = 0; index < input.length; index++) {
      const descriptor = descriptors.get(String(index));
      if (descriptor === undefined) this.fail("SPARSE_ARRAY", `${path}[${index}]`, "array holes are not allowed");
      if (!("value" in descriptor)) this.fail("ACCESSOR", `${path}[${index}]`, "array accessors are not allowed");
      if (!descriptor.enumerable) this.fail("INVALID_VALUE", `${path}[${index}]`, "array entries must be enumerable");
    }
    for (const key of descriptors.keys()) {
      if (key !== "length" && !/^(?:0|[1-9][0-9]*)$/.test(key)) this.fail("UNKNOWN_FIELD", appendPath(path, key), "array properties are not allowed");
    }
    return Array.from({ length: input.length }, (_, index) => descriptors.get(String(index))!.value);
  }

  private mark(value: object, path: string): void {
    if (this.seen.has(value)) this.fail("REPEATED_VALUE", path, "repeated or cyclic object identity");
    this.seen.add(value);
  }

  private peekKind(input: unknown, path: string): string {
    if (input === null || typeof input !== "object" || Array.isArray(input)) this.fail("INVALID_TYPE", path, "plain object required");
    rejectProxy(input, path);
    const descriptor = this.describe(input).get("kind");
    if (descriptor === undefined) this.fail("MISSING_FIELD", `${path}.kind`, "required field is missing");
    if (!("value" in descriptor)) this.fail("ACCESSOR", `${path}.kind`, "accessors are not allowed");
    if (typeof descriptor.value !== "string") this.fail("INVALID_TYPE", `${path}.kind`, "kind must be a string");
    return descriptor.value;
  }

  private identifier(input: unknown, path: string): string {
    const value = this.string(input, path);
    if (value.length > 64 || !IDENTIFIER.test(value)) this.fail("INVALID_IDENTIFIER", path, "invalid identifier");
    return value;
  }

  private string(input: unknown, path: string): string {
    if (typeof input !== "string") this.fail("INVALID_TYPE", path, "string required");
    if (input.length > this.limits.maxStringBytes) this.fail("LIMIT_EXCEEDED", path, "string byte limit exceeded");
    if (encoder.encode(input).byteLength > this.limits.maxStringBytes) this.fail("LIMIT_EXCEEDED", path, "string byte limit exceeded");
    if (!isWellFormedUnicode(input)) this.fail("INVALID_VALUE", path, "string must contain well-formed Unicode scalar values");
    return input;
  }

  private describe(value: object): ReadonlyMap<string, PropertyDescriptor> {
    const descriptors = this.descriptors.get(value);
    if (descriptors === undefined) throw new Error("internal error: value was not preflighted");
    return descriptors;
  }

  private prototypeOf(value: object): object | null {
    const prototype = this.prototypes.get(value);
    if (prototype === undefined && !this.prototypes.has(value)) throw new Error("internal error: value was not preflighted");
    return prototype ?? null;
  }

  private boolean(input: unknown, path: string): boolean {
    if (typeof input !== "boolean") this.fail("INVALID_TYPE", path, "boolean required");
    return input;
  }

  private number(input: unknown, path: string): number {
    if (typeof input !== "number") this.fail("INVALID_TYPE", path, "number required");
    if (!Number.isFinite(input)) this.fail("INVALID_VALUE", path, "number must be finite");
    return input;
  }

  private literal<T extends string | number>(input: unknown, expected: T, path: string): T {
    if (input !== expected) this.fail("INVALID_VALUE", path, `expected ${String(expected)}`);
    return expected;
  }

  private oneOf<T extends string>(input: unknown, options: readonly T[], path: string): T {
    if (typeof input !== "string") this.fail("INVALID_TYPE", path, "string required");
    if (!options.includes(input as T)) this.fail("INVALID_VALUE", path, "unsupported value");
    return input as T;
  }

  private resolveState(name: string, path: string): StateDeclaration {
    const state = this.stateByName.get(name);
    if (state === undefined) this.fail("DANGLING_REFERENCE", path, "unknown state");
    return state;
  }

  private valueMatches(type: StateType, value: JsonValue, values?: readonly string[]): boolean {
    if (type === "json") return true;
    if (type === "enum") return typeof value === "string" && values?.includes(value) === true;
    return typeof value === type;
  }

  private expressionMatches(target: StateDeclaration, expression: ValueExpr): boolean {
    if (expression.kind === "literal") return this.valueMatches(target.type, expression.value, target.values);
    const source = this.stateByName.get(expression.state)!;
    if (target.type === "json") return true;
    if (target.type !== source.type) return false;
    if (target.type !== "enum") return true;
    const targetValues = new Set(target.values);
    return source.values?.every(value => targetValues.has(value)) === true;
  }

  private safeNavigation(to: string): boolean {
    if (to.length === 0 || /[\u0000-\u001f\u007f\\]/.test(to) || to.startsWith("//")) return false;
    if (to.startsWith("/")) return true;
    if (to.startsWith("#")) return to.length > 1;
    try {
      const parsed = new URL(to);
      return parsed.protocol === "https:" && parsed.username === "" && parsed.password === "";
    } catch {
      return false;
    }
  }

  private fail(code: ConstructorParameters<typeof InteractivityError>[0], path: string, message: string): never {
    throw new InteractivityError(code, path, message);
  }
}

export function validateInteractivityDocument(
  input: unknown,
  limits?: Partial<InteractivityLimits>,
): InteractivityDocument {
  return new Validator(snapshotLimits(limits)).validate(input);
}

export const EMPTY_INTERACTIVITY: InteractivityDocument = validateInteractivityDocument({
  kind: "Interactivity",
  version: 1,
  state: [],
  bindings: [],
  handlers: [],
  islands: [],
});
