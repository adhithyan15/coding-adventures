// The Journal runtime in a browser's shoes (J5c-1, #14416).
//
// Loads the real wasm32 build through the standard Mosaic loader
// (`mosaic-host.mjs`), the one the web host uses. It supplies the clock the
// module imports (`journal.now_ms`), then drives the app the way the generated
// React component will: bare event names (`newEntry`), not `onNewEntry`.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { loadMosaicModule } from '../../mosaic-app-wasm/js/mosaic-host.mjs';

const artifact = new URL('../../target/wasm32-unknown-unknown/debug/journal_mosaic_app.wasm', import.meta.url);

// 2026-09-24T12:00:00Z: a fixed, known day for the timeline heading.
const NOW = Date.UTC(2026, 8, 24, 12);

async function load(nowMs = () => NOW) {
  return loadMosaicModule(await readFile(artifact), { journal: { now_ms: nowMs } });
}

test('writes, saves and restores an entry through the Mosaic WASM host', async () => {
  const module = await load();
  const app = module.create({ protocolVersion: 2, colorScheme: 'light' });
  assert.equal(app.update.props['timeline-empty'], true);
  assert.deepEqual(app.update.props['timeline-rows'], []);

  app.dispatch('newEntry');
  app.dispatch('titleChange', { value: 'First light' });
  app.dispatch('bodyChange', { value: 'Wrote this in the browser.' });
  const saved = app.dispatch('saveEntry');
  assert.equal(saved.props['timeline-empty'], false);
  const [row] = saved.props['timeline-rows'];
  assert.equal(row[2], 'First light');
  // Dated from the HOST's clock: the heading is that UTC day.
  assert.equal(row[1], 'Thursday, 24 September 2026');
  assert.equal(saved.props['delete-label'], 'Delete');

  // The raw emit name is still accepted, as native hosts send it.
  const fresh = app.dispatch('onNewEntry');
  assert.equal(fresh.props['draft-title'], '');
  assert.equal(fresh.props['selected-key'], '');

  const snapshot = app.snapshot();
  const restored = (await load()).create({ protocolVersion: 2, colorScheme: 'light' });
  const back = restored.restore(snapshot);
  assert.equal(back.props['timeline-rows'][0][2], 'First light');
});

test('an unknown event is refused and names what was sent', async () => {
  const app = (await load()).create({ protocolVersion: 2, colorScheme: 'light' });
  assert.throws(() => app.dispatch('notAJournalEvent'), /notAJournalEvent/);
});

test('an implausible host clock dates entries at the epoch, never traps', async () => {
  const app = (await load(() => Number.NaN)).create({ protocolVersion: 2, colorScheme: 'light' });
  app.dispatch('newEntry');
  app.dispatch('titleChange', { value: 'No clock' });
  const saved = app.dispatch('saveEntry');
  assert.equal(saved.props['timeline-rows'][0][1], 'Thursday, 1 January 1970');
});

test('the module cannot start without the host clock', async () => {
  await assert.rejects(loadMosaicModule(await readFile(artifact)), /journal|now_ms|import/i);
});
