import type { MosaicHost, MosaicUpdate } from './mosaic-host.mjs';
export const MAX_FILE_BYTES: number;
/** Minimum time between two fallback downloads (no File System Access API). */
export const DOWNLOAD_FALLBACK_INTERVAL_MS: number;
/**
 * UI87 §3.1: a plain file name -- no separators or `:`, no leading `.`, no
 * control/format/line-/paragraph-separator characters, no leading or trailing
 * whitespace or whitespace run, no trailing dot; ≤ 255 UTF-16 units.
 */
export function isPlainFileName(name: unknown): boolean;
/** Extensions that run when opened; refused for a save that names no type. */
export const EXECUTABLE_EXTENSIONS: ReadonlySet<string>;
/** True when `name` ends in one of `EXECUTABLE_EXTENSIONS`. */
export function hasExecutableExtension(name: string): boolean;
export interface MosaicFileEffects {
  /** Run synchronously from the initiating gesture; await the resulting update. */
  run(effect: MosaicUpdate['effects'][number]): Promise<MosaicUpdate | undefined>;
  /** Retry only a rejected completion, without opening a dialog or repeating I/O. */
  retry(id: number): Promise<MosaicUpdate | undefined>;
  /** Dispose this executor before destroying the originating MosaicHost. */
  dispose(): void;
}
/** `environment` defaults to `globalThis`; a test passes a fake with the pickers. */
export function createBrowserFileEffects(host: MosaicHost, environment?: object): MosaicFileEffects;
