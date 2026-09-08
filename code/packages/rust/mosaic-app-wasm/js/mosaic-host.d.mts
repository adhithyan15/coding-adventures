export type MosaicEffectResult = { ok: unknown } | { cancelled: Record<string, never> } | { failed: { message: string } };
export class MosaicHostError extends Error {
  constructor(message: string, code: string, pendingEffects?: number[]);
  code: string;
  pendingEffects: number[];
}
export interface MosaicSnapshot { schema: string; version: number; bytes: number[] }
export interface MosaicUpdate {
  protocolVersion: number;
  revision: number;
  props: Record<string, unknown>;
  effects: Array<{ id: number; kind: string; payload: unknown; delivery?: "notify" | "await" }>;
  announcements: Array<{ politeness: string; message: string }>;
}
export interface MosaicHost {
  readonly update: MosaicUpdate;
  dispatch(name: string, payload?: Record<string, unknown>): MosaicUpdate;
  completeEffect(id: number, result: MosaicEffectResult): MosaicUpdate;
  snapshot(): MosaicSnapshot | null;
  restore(snapshot: MosaicSnapshot): MosaicUpdate;
  dispose(): void;
}
export interface MosaicModule { create(context?: Record<string, unknown>): MosaicHost }
export function loadMosaicModule(bytes: BufferSource | WebAssembly.Module, imports?: WebAssembly.Imports): Promise<MosaicModule>;
export function createMosaicModule(instance: WebAssembly.Instance): MosaicModule;
