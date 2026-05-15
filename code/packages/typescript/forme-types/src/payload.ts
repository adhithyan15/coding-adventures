/**
 * KindPayload — map a runtime KindDescriptor to its compile-time
 * TypeScript value type.
 *
 * Stages are parameterised by `Stage<In, Out>` where `In` and `Out`
 * are KindDescriptor values (runtime objects).  TypeScript infers the
 * concrete value type the stage's `run` method should accept and return
 * by looking up the descriptor's `name` in `KindPayloadMap`.
 *
 * Example:
 *
 *   type T1 = KindPayload<typeof Kinds.ContentSource>; // → ContentSource
 *   type T2 = KindPayload<typeof Kinds.Void>;          // → void
 *
 * For Stream descriptors built with `streamOf(inner)`, the payload is
 * `AsyncIterable<KindPayload<inner>>` — the consumer iterates lazily.
 *
 * Plugin-contributed `ext:*` kinds resolve to `unknown` — they need
 * their own type augmentation to be statically typed.  Augmentation
 * pattern:
 *
 *   declare module "@coding-adventures/forme-types" {
 *     interface KindPayloadMap {
 *       "ext:my-kind": MyKindShape;
 *     }
 *   }
 *
 * Augmentation is *additive* — declaring `KindPayloadMap` in a plugin
 * only adds new keys; existing keys keep their kernel-defined types.
 */

import type {
  Asset, Collection, ContentNode, ContentSource,
  DeployArtifact, Document, Feed,
  PrintForme, RenderedPage, RequestHandler, SearchIndex,
} from "./shapes.js";
import type { KindDescriptor } from "./kinds.js";

/**
 * The mapping from kind name (string) to its TypeScript payload type.
 * Declared as an `interface` (not `type`) so plugins can augment it.
 */
export interface KindPayloadMap {
  Void:           void;
  ContentSource:  ContentSource;
  ContentNode:    ContentNode;
  Collection:     Collection;
  Asset:          Asset;
  Document:       Document;
  RenderedPage:   RenderedPage;
  PrintForme:     PrintForme;
  RequestHandler: RequestHandler;
  SearchIndex:    SearchIndex;
  Feed:           Feed;
  DeployArtifact: DeployArtifact;
}

/**
 * Compile-time mapping from a `KindDescriptor` to its TypeScript
 * payload type.  Stream descriptors resolve to AsyncIterable wrappers;
 * unknown / `ext:*` names resolve to `unknown` (must be narrowed by
 * the consumer or made known via module augmentation).
 *
 * Implementation note: TypeScript can't peek inside `descriptor.inner`
 * at the type level for a runtime value, so the AsyncIterable case
 * resolves to `AsyncIterable<unknown>` here.  Stages that produce or
 * consume streams typically annotate explicitly via `Stage<typeof
 * streamOf(Kinds.X), ...>` and rely on `unknown` being narrowed at the
 * call boundary.  A stricter mapping is possible with conditional types
 * over `K["inner"]` but adds noise without much win — the orchestrator
 * checks descriptor compatibility at runtime regardless.
 */
export type KindPayload<K extends KindDescriptor> =
  K["name"] extends "Stream"
    ? AsyncIterable<unknown>
    : K["name"] extends keyof KindPayloadMap
      ? KindPayloadMap[K["name"]]
      : unknown;
