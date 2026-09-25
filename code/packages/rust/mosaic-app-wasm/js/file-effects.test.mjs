import assert from 'node:assert/strict';
import test from 'node:test';
import { readFile } from 'node:fs/promises';
import { createBrowserFileEffects, MAX_FILE_BYTES } from './mosaic-file-effects.mjs';
import { loadMosaicModule } from './mosaic-host.mjs';

const effect = (id = 1, kind = 'file.open', payload = {}) => ({ id, delivery: 'await', kind, payload });
const deferred = () => {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
};
function fixture(overrides = {}) {
  const calls = [];
  const bytes = Uint8Array.from([0, 1, 127, 128, 255]);
  const environment = {
    isSecureContext: true, navigator: { userActivation: { isActive: true } },
    showOpenFilePicker: async options => {
      calls.push(['open', options]);
      return [{ getFile: async () => ({ name: 'workbook.json', size: bytes.length, arrayBuffer: async () => bytes.buffer }) }];
    },
    showSaveFilePicker: async options => {
      calls.push(['save', options]);
      return { name: options.suggestedName, createWritable: async () => ({
        write: async data => { calls.push(['write', [...data]]); },
        close: async () => { calls.push(['close']); },
        abort: async () => { calls.push(['abort']); },
      }) };
    },
    ...overrides,
  };
  const completions = [];
  const host = { update: { protocolVersion: 2 }, completeEffect(id, result) { completions.push([id, result]); return { revision: completions.length }; } };
  return { calls, completions, environment, host, files: createBrowserFileEffects(host, environment) };
}

test('open preserves arbitrary bytes, invokes picker before yielding and never replays duplicate IDs', async () => {
  const { files, calls, completions } = fixture();
  const pending = files.run(effect(1, 'file.open', { mimeType: 'application/json', extension: '.json' }));
  assert.equal(calls[0][0], 'open', 'picker must start inside the initiating gesture');
  assert.equal(await files.run(effect()), undefined);
  assert.deepEqual(await pending, { revision: 1 });
  assert.deepEqual(completions, [[1, { ok: { name: 'workbook.json', bytes: 'AAF/gP8=' } }]]);
  assert.equal(calls[0][1].multiple, false);
  assert.deepEqual(calls[0][1].types, [{ accept: { 'application/json': ['.json'] } }]);
  assert.equal(await files.run(effect()), undefined);
  assert.equal(await files.retry(1), undefined);
  assert.equal(calls.length, 1);
  assert.equal(completions.length, 1);
});

test('save writes opaque bytes and reports success only after close', async () => {
  const { files, calls, completions } = fixture();
  await files.run(effect(1, 'file.save', { suggestedName: 'budget.json', bytes: 'AAF/gP8=' }));
  assert.deepEqual(calls, [['save', { suggestedName: 'budget.json' }], ['write', [0, 1, 127, 128, 255]], ['close']]);
  assert.deepEqual(completions, [[1, { ok: { name: 'budget.json' } }]]);
});

test('picker dismissal is cancellation; denial and I/O AbortError are failures', async () => {
  for (const name of ['AbortError', 'NotAllowedError', 'SecurityError']) {
    const { files, completions } = fixture({ showOpenFilePicker() { throw new DOMException('denied', name); } });
    await files.run(effect());
    assert.deepEqual(completions[0][1], name === 'AbortError' ? { cancelled: {} } : { failed: { message: 'denied' } });
  }
  const { files, completions } = fixture({ showOpenFilePicker: async () => [{ getFile() { throw new DOMException('read aborted', 'AbortError'); } }] });
  await files.run(effect());
  assert.deepEqual(completions[0][1], { failed: { message: 'read aborted' } });
});

test('unsupported or inactive environments settle explicitly without opening dialogs', async () => {
  assert.throws(() => createBrowserFileEffects({ update: { protocolVersion: 1 } }), /protocol 2/);
  for (const overrides of [{ isSecureContext: false }, { showOpenFilePicker: undefined },
    { navigator: {} }, { navigator: { userActivation: { isActive: false } } }]) {
    const { files, calls, completions } = fixture(overrides);
    await files.run(effect());
    assert.equal(calls.length, 0);
    assert.ok(completions[0][1].failed.message);
  }
  const { files, calls, completions } = fixture();
  await files.run(effect(1, 'unknown.capability'));
  assert.match(completions[0][1].failed.message, /Unsupported/);
  await assert.rejects(files.run({ ...effect(2), delivery: 'notify' }), /Await/);
  assert.equal(calls.length, 0);
});

test('payload limits reject malformed bytes and oversized reads without writing', async () => {
  for (const payload of [{ suggestedName: '../budget.json', bytes: '' },
    { suggestedName: 'budget.json', bytes: 'not base64!' },
    { suggestedName: 'budget.json', bytes: 'AAAA'.repeat(Math.ceil(MAX_FILE_BYTES / 3) + 1) }]) {
    const { files, calls, completions } = fixture();
    await files.run(effect(1, 'file.save', payload));
    assert.ok(completions[0][1].failed);
    assert.ok(calls.every(([name]) => name !== 'write'));
  }
  let read = false;
  const { files, completions } = fixture({ showOpenFilePicker: async () => [{ getFile: async () => ({
    size: MAX_FILE_BYTES + 1, arrayBuffer() { read = true; },
  }) }] });
  await files.run(effect());
  assert.equal(read, false);
  assert.match(completions[0][1].failed.message, /16 MiB/);
});

test('write and close failures abort the stream and do not report save success', async () => {
  for (const failing of ['write', 'close']) {
    const operations = [];
    const { files, completions } = fixture({ showSaveFilePicker: async () => ({ createWritable: async () => ({
      async write() { operations.push('write'); if (failing === 'write') throw new Error('write failed'); },
      async close() { operations.push('close'); if (failing === 'close') throw new Error('close failed'); },
      async abort() { operations.push('abort'); throw new Error('cleanup also failed'); },
    }) }) });
    await files.run(effect(1, 'file.save', { suggestedName: 'budget.json', bytes: 'AA==' }));
    assert.equal(operations.at(-1), 'abort');
    assert.deepEqual(completions[0][1], { failed: { message: `${failing} failed` } });
  }
});

test('a maximum-size valid payload stays within the transport limit', async () => {
  let written = 0;
  const { files, completions } = fixture({ showSaveFilePicker: async () => ({ name: 'large.bin', createWritable: async () => ({
    async write(bytes) { written = bytes.length; assert.equal(bytes[bytes.length - 1], 0); },
    async close() {}, async abort() { assert.fail('valid bytes must not abort'); },
  }) }) });
  await files.run(effect(1, 'file.save', { suggestedName: 'large.bin', bytes: Buffer.alloc(MAX_FILE_BYTES).toString('base64') }));
  assert.equal(written, MAX_FILE_BYTES);
  assert.deepEqual(completions[0][1], { ok: { name: 'large.bin' } });
});

test('disposal ignores outstanding pickers and aborts writes before commit', async () => {
  const picker = deferred();
  const first = fixture({ showOpenFilePicker: () => picker.promise });
  const pending = first.files.run(effect());
  first.files.dispose();
  picker.resolve([{ getFile() { assert.fail('disposed picker must not be read'); } }]);
  assert.equal(await pending, undefined);
  assert.deepEqual(first.completions, []);
  const write = deferred();
  let aborted = false;
  const second = fixture({ showSaveFilePicker: async () => ({ createWritable: async () => ({
    write: () => write.promise, close() { assert.fail('disposed write must not commit'); },
    async abort() { aborted = true; },
  }) }) });
  const saving = second.files.run(effect(1, 'file.save', { suggestedName: 'budget.json', bytes: '' }));
  await new Promise(resolve => setImmediate(resolve));
  second.files.dispose(); write.resolve();
  assert.equal(await saving, undefined);
  assert.equal(aborted, true);
  assert.deepEqual(second.completions, []);
  assert.equal(await second.files.retry(1), undefined);
});

test('concurrent file requests fail explicitly and completion retry does not repeat I/O', async () => {
  const picker = deferred();
  const first = fixture({ showOpenFilePicker: () => picker.promise });
  const pending = first.files.run(effect());
  await first.files.run(effect(2));
  assert.match(first.completions[0][1].failed.message, /in progress/);
  picker.resolve([]); await pending;
  const second = fixture();
  let attempts = 0;
  const files = createBrowserFileEffects({ update: { protocolVersion: 2 }, completeEffect(id, result) {
    attempts++;
    if (attempts === 1) throw new Error('app rejected result');
    return second.host.completeEffect(id, result);
  } }, second.environment);
  await assert.rejects(files.run(effect()), /app rejected/);
  assert.equal(await files.run(effect()), undefined);
  assert.deepEqual(await files.retry(1), { revision: 1 });
  assert.equal(second.calls.length, 1);
  assert.equal(attempts, 2);
});

test('compiled Mosaic keeps rejected file results pending and accepts explicit cancellation', async () => {
  const artifact = new URL('../../target/wasm32-unknown-unknown/debug/mosaic_app_conformance.wasm', import.meta.url);
  const module = await loadMosaicModule(await readFile(artifact));
  const app = module.create({ protocolVersion: 2 });
  const requested = app.dispatch('requestEffect').effects[0];
  const { environment, calls } = fixture();
  const files = createBrowserFileEffects(app, environment);
  // The counter fixture rejects a file payload: the real Rust boundary must
  // retain Await state, while retrying must never read the file a second time.
  await assert.rejects(files.run({ ...requested, kind: 'file.open' }), /integer/);
  await assert.rejects(files.retry(requested.id), /integer/);
  assert.equal(calls.length, 1);
  assert.equal(app.update.revision, 2);
  assert.throws(() => app.snapshot(), /pending/);
  files.dispose(); app.dispose();
  const next = module.create({ protocolVersion: 2 });
  const cancelled = next.dispatch('requestEffect').effects[0];
  const cancellation = createBrowserFileEffects(next, fixture({ showOpenFilePicker() { throw new DOMException('dismissed', 'AbortError'); } }).environment);
  assert.equal((await cancellation.run({ ...cancelled, kind: 'file.open' })).revision, 3);
  assert.ok(next.snapshot());
  cancellation.dispose(); next.dispose();
});

// UI87 §7: the standard kinds every Mosaic backend answers.

test('files.open filters by UI59 accept types and reports the MIME type', async () => {
  const { files, calls, completions } = fixture();
  await files.run(effect(7, 'files.open', { accept: ['application/json', 'unknown/type'] }));
  assert.deepEqual(calls[0][1].types, [{ accept: { 'application/json': ['.json'] } }]);
  assert.deepEqual(completions, [[7, { ok: { name: 'workbook.json', mimeType: 'application/json', bytes: 'AAF/gP8=' } }]]);
});

test('files.save writes under a plain name that matches the accepted type', async () => {
  const { files, calls, completions } = fixture();
  await files.run(effect(8, 'files.save', { suggestedName: 'journal.json', accept: ['application/json'], bytes: 'AAF/gP8=' }));
  assert.equal(calls[0][1].suggestedName, 'journal.json');
  assert.deepEqual(calls[0][1].types, [{ accept: { 'application/json': ['.json'] } }]);
  assert.deepEqual(calls.slice(1), [['write', [0, 1, 127, 128, 255]], ['close']]);
  assert.deepEqual(completions, [[8, { ok: { name: 'journal.json' } }]]);
});

test('files.save refuses names that are paths, disguises, or the wrong type, before any dialog', async () => {
  for (const [name, accept] of [
    ['../x.json', undefined], ['a/b.json', undefined], ['D:x.json', undefined], ['x.json:s', undefined],
    ['invoice‮fdp.exe', undefined], ['trailing.', undefined], ['trailing ', undefined],
    ['notes.exe', ['application/json']],
  ]) {
    const { files, calls, completions } = fixture();
    await files.run(effect(9, 'files.save', { suggestedName: name, ...(accept ? { accept } : {}), bytes: 'AA==' }));
    assert.equal(calls.length, 0, `no dialog for ${JSON.stringify(name)}`);
    assert.ok(completions[0][1].failed, `failed for ${JSON.stringify(name)}`);
  }
});

test('without the File System Access API, files.save downloads the bytes under the suggested name', async () => {
  const clicked = [];
  const { files, completions } = fixture({
    showSaveFilePicker: undefined,
    Blob: class { constructor(parts) { this.parts = parts; } },
    URL: { createObjectURL: () => 'blob:mosaic', revokeObjectURL: () => {} },
    setTimeout: () => {},
    document: { createElement: tag => ({ tag, click() { clicked.push({ href: this.href, download: this.download }); } }) },
  });
  await files.run(effect(10, 'files.save', { suggestedName: 'journal.json', accept: ['application/json'], bytes: 'AAF/gP8=' }));
  assert.deepEqual(clicked, [{ href: 'blob:mosaic', download: 'journal.json' }]);
  assert.deepEqual(completions, [[10, { ok: { name: 'journal.json', download: true } }]]);

  // The older alias keeps its explicit degradation: no download claims a save.
  const legacy = fixture({ showSaveFilePicker: undefined, document: { createElement: () => ({ click() {} }) },
    URL: { createObjectURL: () => 'blob:x', revokeObjectURL() {} }, Blob: class {} });
  await legacy.files.run(effect(11, 'file.save', { suggestedName: 'book.json', bytes: 'AA==' }));
  assert.ok(legacy.completions[0][1].failed);
});
