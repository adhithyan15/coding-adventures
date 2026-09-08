import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import test from 'node:test';
import { loadMosaicModule, MosaicHostError } from './mosaic-host.mjs';

test('protocol 2 completion preserves pending work, sequence and instance lifetime', async () => {
  const module = await loadMosaicModule(await readFile(artifact('mosaic_app_conformance')));
  const legacy = module.create();
  assert.throws(() => legacy.dispatch('requestEffect'), /protocol 2/);
  assert.equal(legacy.dispatch('increment', { amount: 1 }).props.count, 1);
  legacy.dispose();
  const app = module.create({ protocolVersion: 2 });
  const saved = app.snapshot();
  const effect = app.dispatch('requestEffect').effects[0];
  assert.equal(effect.delivery, 'await');
  const pending = error => error instanceof MosaicHostError && error.code === 'pendingEffects'
    && error.pendingEffects.length === 1 && error.pendingEffects[0] === effect.id;
  assert.throws(() => app.snapshot(), pending);
  assert.throws(() => app.restore(saved), pending);
  for (const result of [{ ok: { amount: 'bad' } }, { cancelled: { extra: true } },
    { ok: {}, cancelled: {} }, { failed: {} }]) {
    assert.throws(() => app.completeEffect(effect.id, result));
    assert.equal(app.update.revision, 2);
    assert.throws(() => app.snapshot(), pending);
  }
  assert.throws(() => app.completeEffect(Number.MAX_SAFE_INTEGER + 1, { cancelled: {} }), /safe integer/);
  assert.throws(() => app.completeEffect(effect.id + 1, { ok: { amount: 9 } }), /unknown|pending/);
  const chained = app.completeEffect(effect.id, { ok: { amount: 7, chain: true } });
  assert.equal(chained.props.count, 7);
  assert.equal(chained.revision, 3);
  assert.throws(() => app.completeEffect(effect.id, { cancelled: {} }), /unknown|pending/);
  const second = chained.effects[0].id;
  assert.ok(second > effect.id);
  assert.throws(() => app.snapshot(), /pending/);
  app.completeEffect(second, { cancelled: {} });
  assert.equal(app.dispatch('increment', { amount: 1 }).props.count, 8, 'completions do not consume UI sequence');
  app.restore(saved);
  const third = app.dispatch('requestEffect').effects[0].id;
  assert.ok(third > second, 'restore does not recycle effect IDs');
  assert.equal(app.completeEffect(third, { failed: { message: 'disk full' } }).props.count, 0);
  const notify = app.dispatch('requestEffect', { notify: true }).effects[0];
  assert.equal(notify.delivery, 'notify');
  assert.deepEqual(app.snapshot(), saved);
  assert.throws(() => app.completeEffect(notify.id, { cancelled: {} }), /unknown|pending/);
  const restored = module.create({ protocolVersion: 2, restoredSnapshot: saved });
  assert.deepEqual(restored.update.effects, []);
  app.dispose();
  assert.throws(() => app.completeEffect(third, { cancelled: {} }), /disposed/);
  assert.equal(restored.dispatch('increment', { amount: 2 }).props.count, 2);
  restored.dispose();
});

const artifact = name => new URL(`../../target/wasm32-unknown-unknown/debug/${name}.wasm`, import.meta.url);

test('compiled transport rejects invalid pointers and JSON, and releases response buffers', async () => {
  const { instance } = await WebAssembly.instantiate(await readFile(artifact('mosaic_app_conformance')));
  const ex = instance.exports;
  assert.equal(ex.mosaic_wasm_alloc(0), 0);
  assert.equal(ex.mosaic_wasm_alloc(64 * 1024 * 1024 + 1), 0);
  assert.equal(ex.mosaic_wasm_call(123), 0);
  ex.mosaic_wasm_free(123);
  function raw(text) {
    const bytes = new TextEncoder().encode(text);
    const input = ex.mosaic_wasm_alloc(bytes.length);
    new Uint8Array(ex.memory.buffer, input, bytes.length).set(bytes);
    const output = ex.mosaic_wasm_call(input);
    const length = new DataView(ex.memory.buffer).getUint32(output, true);
    const response = JSON.parse(new TextDecoder().decode(new Uint8Array(ex.memory.buffer, output + 4, length)));
    ex.mosaic_wasm_free(output);
    return response;
  }
  assert.equal(raw('{').ok, false);
  assert.match(raw('{"op":"snapshot","handle":1}').error, /unknown application handle/);
  const context = { protocolVersion: 1, platform: 'web', locale: 'en', colorScheme: 'light', textScale: 1 };
  const created = raw(JSON.stringify({ op: 'create', context })).value;
  assert.equal(raw(JSON.stringify({ op: 'destroy', handle: created.handle })).ok, true);
  assert.equal(raw(JSON.stringify({ op: 'snapshot', handle: created.handle })).ok, false);
  const next = raw(JSON.stringify({ op: 'create', context })).value;
  assert.notEqual(next.handle, created.handle);
  raw(JSON.stringify({ op: 'destroy', handle: next.handle }));
  const malformed = '{' + ' '.repeat(4096);
  for (let i = 0; i < 100; i++) raw(malformed);
  const warmedSize = ex.memory.buffer.byteLength;
  for (let i = 0; i < 2000; i++) assert.equal(raw(malformed).ok, false);
  assert.equal(ex.memory.buffer.byteLength, warmedSize, 'transport allocations must be reclaimed');
});

test('compiled standard runtime: independent instances, retry, restore and teardown', async () => {
  const module = await loadMosaicModule(await readFile(artifact('mosaic_app_conformance')));
  const a = module.create();
  const b = module.create();
  assert.equal(a.update.props.platform, 'web');
  assert.equal(a.update.revision, 1);
  assert.throws(() => a.dispatch('increment', { amount: 'bad' }), /integer/);
  assert.equal(a.update.revision, 1);
  assert.equal(a.dispatch('increment', { amount: 7 }).props.count, 7);
  assert.equal(a.update.revision, 2);
  assert.equal(b.update.props.count, 0);
  const snapshot = a.snapshot();
  assert.throws(() => a.restore({ ...snapshot, version: 99 }), /snapshot/);
  assert.equal(a.update.revision, 2);
  assert.equal(b.restore(snapshot).props.count, 7);
  assert.equal(b.dispatch('increment', { amount: 2 }).props.count, 9);
  const c = module.create({ restoredSnapshot: snapshot });
  assert.equal(c.update.props.count, 7);
  a.dispose(); a.dispose();
  assert.throws(() => a.snapshot(), /disposed/);
  assert.equal(b.dispatch('increment', { amount: 1 }).props.count, 10);
  b.dispose(); c.dispose();
  assert.throws(() => module.create({ protocolVersion: 99 }), /protocol/);
  const d = module.create();
  assert.equal(d.update.props.count, 0);
  d.dispose();
});

test('compiled VisiCalc replays the shared presentation contract and restores committed work', async () => {
  const module = await loadMosaicModule(await readFile(artifact('visicalc_mosaic_app')));
  const app = module.create();
  const fixture = JSON.parse(await readFile(new URL('../../../../programs/mosaic/visicalc/fixtures/presentation-contract-v1.json', import.meta.url)));
  const names = { selectedRow: 'selected-row', selectedCol: 'selected-col',
    viewportOffset: 'viewport-offset', viewportSize: 'viewport-size', formula: 'formula', editing: 'editing' };
  for (const step of fixture.steps) {
    if (step.event) {
      const { type, payload } = step.event;
      app.dispatch(type, payload);
    }
    for (const [slot, value] of Object.entries(step.expected.slots)) {
      assert.deepEqual(app.update.props[names[slot]], value, `${step.id}: ${slot}`);
    }
  }
  const saved = app.snapshot();
  app.dispatch('formulaChange', { value: '999' });
  const restored = module.create({ restoredSnapshot: saved });
  assert.equal(restored.update.props.formula, '20');
  assert.equal(restored.update.props['viewport-rows'][4][4], '174');
  assert.equal(restored.update.props.editing, false);
  app.dispose(); restored.dispose();
});
