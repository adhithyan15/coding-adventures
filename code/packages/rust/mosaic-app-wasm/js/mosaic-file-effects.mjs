// Browser capability execution only: the Rust app owns file formats and state.
export const MAX_FILE_BYTES = 16 * 1024 * 1024;

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

function pickerOptions(payload) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    throw new Error('File effect payload must be an object');
  }
  const options = {};
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
export function createBrowserFileEffects(host, environment = globalThis) {
  if (host.update.protocolVersion !== 2) throw new Error('Browser file effects require protocol 2');
  let disposed = false;
  let busy = false;
  const records = new Map();

  async function perform(effect) {
    if (busy) return { failed: { message: 'Another file operation is still in progress' } };
    busy = true;
    let writer;
    let closed = false;
    let picking = false;
    try {
      const options = pickerOptions(effect.payload);
      const saving = effect.kind === 'file.save';
      const savedBytes = effect.payload.bytes;
      if (!saving && effect.kind !== 'file.open') throw new Error(`Unsupported browser capability: ${effect.kind}`);
      const picker = saving ? environment.showSaveFilePicker : environment.showOpenFilePicker;
      if (environment.isSecureContext !== true || typeof picker !== 'function') {
        throw new Error('File dialogs are unavailable in this browser or context');
      }
      if (environment.navigator?.userActivation?.isActive !== true) {
        throw new Error('Open or save a file from a fresh user gesture');
      }
      // Validate inexpensive metadata before the picker. Decode bytes only after
      // it opens so large payload processing cannot consume transient activation.
      if (saving) {
        const name = effect.payload.suggestedName;
        if (typeof name !== 'string' || !name.trim() || name.length > 255 || /[\\/\u0000]/.test(name)) {
          throw new Error('Save requires a plain suggested file name');
        }
        if (typeof savedBytes !== 'string' || savedBytes.length > Math.ceil(MAX_FILE_BYTES / 3) * 4) {
          throw new Error('File bytes must be base64 and no larger than 16 MiB');
        }
        options.suggestedName = name;
      } else { options.multiple = false; }
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
      return { ok: { name: file.name, bytes: encode(new Uint8Array(buffer)) } };
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
