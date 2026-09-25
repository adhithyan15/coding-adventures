// Browser capability execution only: the Rust app owns file formats and state.
//
// This is the web's platform library (UI87 §7.1). It answers the standard
// kinds every Mosaic backend answers with the same payloads and results:
//
//   files.open { accept: [MIME...] }
//     -> ok { name, mimeType, bytes } | cancelled {} | failed { message }
//   files.save { suggestedName, accept: [MIME...], bytes }
//     -> ok { name } | cancelled {} | failed { message }
//
// `file.open` / `file.save`, the names this executor answered first, stay as
// aliases until VisiCalc migrates (UI87 §7.3), and their `mimeType` +
// `extension` pair is still accepted.
export const MAX_FILE_BYTES = 16 * 1024 * 1024;

const OPEN_KINDS = new Set(['files.open', 'file.open']);
const SAVE_KINDS = new Set(['files.save', 'file.save']);

// MIME types an app names, and the extensions a picker filters on. Unknown
// types are dropped rather than failing the request (UI59 §3). The same table
// as the Compose library, so every backend filters alike.
const MIME_EXTENSIONS = {
  'application/json': ['.json'],
  'text/plain': ['.txt'],
  'text/markdown': ['.md', '.markdown'],
  'text/csv': ['.csv'],
  'application/pdf': ['.pdf'],
  'application/zip': ['.zip'],
  'image/jpeg': ['.jpg', '.jpeg'],
  'image/png': ['.png'],
  'image/webp': ['.webp'],
  'image/gif': ['.gif'],
  'image/bmp': ['.bmp'],
  'image/tiff': ['.tif', '.tiff'],
  'image/svg+xml': ['.svg'],
};

function encode(bytes) {
  let binary = '';
  for (let offset = 0; offset < bytes.length; offset += 8192) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
  }
  return btoa(binary);
}

function decode(value) {
  if (typeof value !== 'string' || value.length > Math.ceil(MAX_FILE_BYTES / 3) * 4
      || value.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(value)) {
    throw new Error('File bytes must be base64 and no larger than 16 MiB');
  }
  const binary = atob(value);
  if (binary.length > MAX_FILE_BYTES) throw new Error('File exceeds 16 MiB');
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
}

/** The accepted types as `{ MIME: [extensions] }`, from UI59's `accept` list. */
function acceptedTypes(payload) {
  if (payload.accept === undefined) return {};
  if (!Array.isArray(payload.accept) || payload.accept.length > 64) {
    throw new Error('accept must be a list of MIME types');
  }
  const accept = {};
  for (const mime of payload.accept) {
    if (typeof mime === 'string' && Object.hasOwn(MIME_EXTENSIONS, mime)) {
      accept[mime] = MIME_EXTENSIONS[mime];
    }
  }
  return accept;
}

/**
 * A suggested name is a plain file name (UI87 §3.1): no separators, no `:`
 * (a Windows drive or alternate data stream), no control or format characters
 * (a right-to-left override can disguise `.exe` as `.pdf`), no trailing dot or
 * space, at most 255 characters. The same rule as the Compose library.
 */
export function isPlainFileName(name) {
  return typeof name === 'string'
    && name.trim() !== ''
    && name.length <= 255
    && name !== '.' && name !== '..'
    && !/[.\s]$/.test(name)
    // Runs of spaces are how `Invoice.pdf<30 spaces>.exe` hides its extension.
    && !/\s{2,}/.test(name)
    && !/[\\/:\u0000-\u001f\u007f-\u009f\p{Cf}]/u.test(name);
}

function mimeTypeFor(file) {
  if (typeof file.type === 'string' && /^[\w.+-]+\/[\w.+-]+$/.test(file.type)) return file.type;
  const lower = String(file.name ?? '').toLowerCase();
  for (const [mime, extensions] of Object.entries(MIME_EXTENSIONS)) {
    if (extensions.some(extension => lower.endsWith(extension))) return mime;
  }
  return 'application/octet-stream';
}

function pickerOptions(payload) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    throw new Error('File effect payload must be an object');
  }
  const options = {};
  const accept = acceptedTypes(payload);
  if (Object.keys(accept).length > 0) options.types = [{ accept }];
  if (payload.mimeType !== undefined || payload.extension !== undefined) {
    if (typeof payload.mimeType !== 'string' || !/^[\w.+-]+\/[\w.+-]+$/.test(payload.mimeType)
        || typeof payload.extension !== 'string' || !/^\.[A-Za-z0-9.]{1,15}$/.test(payload.extension)) {
      throw new Error('File type requires a MIME type and extension');
    }
    options.types = [{ accept: { [payload.mimeType]: [payload.extension] } }];
  }
  return options;
}

/** One executor per live MosaicHost. Call run directly in the UI gesture,
 * before yielding to React effects, timers or other asynchronous work. */
/** At most one fallback download in this window: without a picker the person
 * never confirms a download, so one gesture must not start a burst of them. */
export const DOWNLOAD_FALLBACK_INTERVAL_MS = 5_000;

export function createBrowserFileEffects(host, environment = globalThis) {
  if (host.update.protocolVersion !== 2) throw new Error('Browser file effects require protocol 2');
  let disposed = false;
  let busy = false;
  let lastFallbackDownload = -Infinity;
  const records = new Map();

  async function perform(effect) {
    if (busy) return { failed: { message: 'Another file operation is still in progress' } };
    busy = true;
    let writer;
    let closed = false;
    let picking = false;
    try {
      const saving = SAVE_KINDS.has(effect.kind);
      if (!saving && !OPEN_KINDS.has(effect.kind)) throw new Error(`Unsupported browser capability: ${effect.kind}`);
      const options = pickerOptions(effect.payload);
      const savedBytes = effect.payload.bytes;
      // Validate inexpensive metadata before the picker. Decode bytes only after
      // it opens so large payload processing cannot consume transient activation.
      if (saving) {
        const name = effect.payload.suggestedName;
        if (!isPlainFileName(name)) throw new Error('Save requires a plain suggested file name');
        // When the app says what it is saving, the name must agree: a JSON
        // export cannot be offered as `notes.exe`.
        const extensions = Object.values(acceptedTypes(effect.payload)).flat();
        if (extensions.length > 0 && !extensions.some(extension => name.toLowerCase().endsWith(extension))) {
          throw new Error('suggestedName must end in an extension of an accepted type');
        }
        if (typeof savedBytes !== 'string' || savedBytes.length > Math.ceil(MAX_FILE_BYTES / 3) * 4) {
          throw new Error('File bytes must be base64 and no larger than 16 MiB');
        }
        options.suggestedName = name;
      } else { options.multiple = false; }
      const picker = saving ? environment.showSaveFilePicker : environment.showOpenFilePicker;
      if (environment.isSecureContext !== true) {
        throw new Error('File dialogs are unavailable in this browser or context');
      }
      if (environment.navigator?.userActivation?.isActive !== true) {
        throw new Error('Open or save a file from a fresh user gesture');
      }
      if (typeof picker !== 'function') {
        // No File System Access API (Firefox, Safari). `files.save` falls back
        // to a download of the same bytes under the suggested name. Without a
        // picker the person never sees or confirms the name, so the fallback
        // is narrower than the picker path:
        //  - only for a known accepted type, and a name ending in its
        //    extension -- never an `.exe` or `.bat` with no `accept`;
        //  - at most one download per DOWNLOAD_FALLBACK_INTERVAL_MS, so one
        //    gesture cannot start a burst;
        //  - the Blob is typed as the accepted MIME type.
        // A download cannot claim the file was durably saved where the person
        // chose, so the result says `download: true`, and `name` is the name
        // requested (the browser may rename or block it). The older `file.save`
        // alias keeps degrading explicitly, as before.
        const accepted = Object.entries(acceptedTypes(effect.payload));
        const match = accepted.find(([, extensions]) =>
          extensions.some(extension => options.suggestedName?.toLowerCase().endsWith(extension)));
        const now = typeof environment.performance?.now === 'function' ? environment.performance.now() : Date.now();
        if (effect.kind === 'files.save' && match
            && typeof environment.document?.createElement === 'function'
            && typeof environment.URL?.createObjectURL === 'function') {
          if (now - lastFallbackDownload < DOWNLOAD_FALLBACK_INTERVAL_MS) {
            throw new Error('Only one download at a time; try again in a moment');
          }
          lastFallbackDownload = now;
          const bytes = decode(savedBytes);
          const url = environment.URL.createObjectURL(new environment.Blob([bytes], { type: match[0] }));
          const revoke = () => environment.URL.revokeObjectURL(url);
          try {
            const link = environment.document.createElement('a');
            link.href = url;
            link.download = options.suggestedName;
            link.rel = 'noopener';
            link.click();
          } finally {
            const later = environment.setTimeout ?? globalThis.setTimeout;
            if (typeof later === 'function') later(revoke, 60_000); else revoke();
          }
          return { ok: { name: options.suggestedName, download: true } };
        }
        throw new Error('File dialogs are unavailable in this browser or context');
      }
      picking = true;
      const chosen = await picker.call(environment, options);
      picking = false;
      if (disposed) return;
      if (saving) {
        const bytes = decode(savedBytes);
        writer = await chosen.createWritable();
        if (disposed) return;
        await writer.write(bytes);
        if (disposed) return;
        await writer.close();
        closed = true;
        if (disposed) return;
        return { ok: { name: chosen.name } };
      }
      if (!Array.isArray(chosen) || chosen.length !== 1) throw new Error('Choose exactly one file');
      const file = await chosen[0].getFile();
      if (disposed) return;
      if (!Number.isSafeInteger(file.size) || file.size < 0 || file.size > MAX_FILE_BYTES) throw new Error('File exceeds 16 MiB');
      const buffer = await file.arrayBuffer();
      if (disposed) return;
      if (buffer.byteLength > MAX_FILE_BYTES) throw new Error('File exceeds 16 MiB');
      // UI59's result carries `mimeType`; the older `file.open` alias keeps its
      // original shape exactly, so VisiCalc sees no change.
      return effect.kind === 'files.open'
        ? { ok: { name: file.name, mimeType: mimeTypeFor(file), bytes: encode(new Uint8Array(buffer)) } }
        : { ok: { name: file.name, bytes: encode(new Uint8Array(buffer)) } };
    } catch (error) {
      if (disposed) return;
      if (picking && error?.name === 'AbortError') return { cancelled: {} };
      return { failed: { message: String(error?.message ?? error).slice(0, 2048) } };
    } finally {
      if (writer && !closed) {
        // Do not commit an interrupted write. Preserve the original failure if
        // abort itself fails (for example, the browser already closed the stream).
        try { await writer.abort(); } catch { /* best-effort stream cleanup */ }
      }
      busy = false;
    }
  }

  function deliver(id, record) {
    try {
      if (disposed || !record.outcome) return;
      const update = host.completeEffect(id, record.outcome);
      record.done = true;
      record.outcome = undefined; // release file bytes, retain duplicate guard
      return update;
    } finally { record.active = false; }
  }

  return {
    async run(effect) {
      if (disposed) return;
      if (!Number.isSafeInteger(effect?.id) || effect.id <= 0 || effect.delivery !== 'await') {
        throw new Error('File capabilities require a protocol-2 Await effect');
      }
      if (records.has(effect.id)) return; // neither repeat I/O nor replay an old render
      const record = { active: true, done: false, outcome: undefined };
      records.set(effect.id, record);
      record.outcome = await perform(effect);
      return deliver(effect.id, record);
    },
    async retry(id) {
      if (disposed) return;
      const record = records.get(id);
      if (!record || record.done || record.active) return;
      record.active = true;
      return deliver(id, record);
    },
    dispose() {
      disposed = true;
      records.clear();
    },
  };
}
