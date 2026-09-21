import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { loadMosaicModule } from '../../mosaic-app-wasm/js/mosaic-host.mjs';

const artifact = new URL('../../target/wasm32-unknown-unknown/debug/spice_mosaic_app.wasm', import.meta.url);

const schematic = {
  title: 'WASM RC',
  components: [
    { reference: 'V1', kind: 'DcVoltage', value: '5', terminals: [{ x: 0, y: 20 }, { x: 0, y: 0 }] },
    { reference: 'R1', kind: 'Resistor', value: '1k', terminals: [{ x: 0, y: 20 }, { x: 40, y: 20 }] },
    { reference: 'G1', kind: 'Ground', value: '', terminals: [{ x: 0, y: 0 }] },
  ],
  wires: [],
};

test('compiled SPICE workbench preserves its schematic lifecycle through the Mosaic WASM host', async () => {
  const module = await loadMosaicModule(await readFile(artifact));
  const app = module.create({ protocolVersion: 2, colorScheme: 'dark' });

  assert.equal(app.update.props['mode-label'], 'Inspection');
  assert.equal(app.update.props['analysis-rows'].length, 5);
  assert.equal(app.update.props['dark-theme'], true);

  const loaded = app.dispatch('schematicLoad', { document: schematic });
  assert.equal(loaded.props['mode-label'], 'Schematic');
  assert.deepEqual(loaded.props['schematic-rows'], ['V1', 'R1', 'G1']);

  const save = app.dispatch('onSaveSchematic');
  const effect = save.effects[0];
  assert.equal(effect.kind, 'file.save');
  assert.equal(effect.delivery, 'await');
  assert.equal(effect.payload.suggestedName, 'wasm-rc.spice-mosaic.json');
  assert.match(effect.payload.bytes, /^[A-Za-z0-9+/]+={0,2}$/);
  const exported = app.completeEffect(effect.id, { ok: { name: effect.payload.suggestedName } });
  assert.equal(exported.props.diagnostics, 'Schematic exported.');

  const snapshot = app.snapshot();
  const synchronized = app.dispatch('onSynchronizeSchematic');
  assert.equal(synchronized.props['mode-label'], 'Inspection');
  assert.equal(
    synchronized.props['netlist-text'],
    '* WASM RC\nR1 n1 n2 1k\nV1 n1 0 DC 5\n.op\n.end\n',
  );

  const restored = module.create({ protocolVersion: 2, restoredSnapshot: snapshot });
  assert.equal(restored.update.props['mode-label'], 'Draft');
  assert.deepEqual(restored.update.props['schematic-rows'], ['V1', 'R1', 'G1']);
  app.dispose();
  restored.dispose();
  assert.throws(() => app.snapshot(), /disposed/);
});
