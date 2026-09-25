import type { MosaicHost, MosaicUpdate } from './mosaic-host.mjs';
export const MAX_FILE_BYTES: number;
/** UI87 §3.1: a plain file name (no separators, `:`, control/format characters, trailing dot or space; ≤ 255). */
export function isPlainFileName(name: unknown): boolean;
export interface MosaicFileEffects {
  /** Run synchronously from the initiating gesture; await the resulting update. */
  run(effect: MosaicUpdate['effects'][number]): Promise<MosaicUpdate | undefined>;
  /** Retry only a rejected completion, without opening a dialog or repeating I/O. */
  retry(id: number): Promise<MosaicUpdate | undefined>;
  /** Dispose this executor before destroying the originating MosaicHost. */
  dispose(): void;
}
export function createBrowserFileEffects(host: MosaicHost): MosaicFileEffects;
