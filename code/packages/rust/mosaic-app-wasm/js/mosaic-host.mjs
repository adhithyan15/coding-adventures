// Standard Mosaic lifecycle transport. No application behavior belongs here.
export async function loadMosaicModule(bytes, imports = {}) {
  const result = await WebAssembly.instantiate(bytes, imports);
  return createMosaicModule(result.instance ?? result);
}

export function createMosaicModule(instance) {
  const ex = instance.exports;
  for (const name of ['mosaic_wasm_alloc', 'mosaic_wasm_call', 'mosaic_wasm_free']) {
    if (typeof ex[name] !== 'function') throw new Error(`Missing Mosaic export: ${name}`);
  }
  if (!(ex.memory instanceof WebAssembly.Memory)) throw new Error('Missing Mosaic memory');
  const encoder = new TextEncoder();
  const decoder = new TextDecoder('utf-8', { fatal: true });
  let failed = false;

  function request(value) {
    if (failed) throw new Error('Mosaic module trapped; reload it before continuing');
    const bytes = encoder.encode(JSON.stringify(value));
    let input = 0;
    let output = 0;
    try {
      input = ex.mosaic_wasm_alloc(bytes.length) >>> 0;
      if (!input) throw new Error('Mosaic input allocation failed');
      new Uint8Array(ex.memory.buffer, input, bytes.length).set(bytes);
      const consumed = input;
      input = 0; // call takes ownership, including on a WASM trap
      output = ex.mosaic_wasm_call(consumed) >>> 0;
      if (!output) throw new Error('Mosaic transport returned no response');
      const length = new DataView(ex.memory.buffer).getUint32(output, true);
      const response = JSON.parse(decoder.decode(new Uint8Array(ex.memory.buffer, output + 4, length)));
      if (!response.ok) throw new Error(response.error);
      return response.value;
    } catch (error) {
      if (error instanceof WebAssembly.RuntimeError) failed = true;
      throw error;
    } finally {
      if (!failed) {
        if (input) ex.mosaic_wasm_free(input);
        if (output) ex.mosaic_wasm_free(output);
      }
    }
  }

  return {
    create(context = {}) {
      const created = request({ op: 'create', context: {
        protocolVersion: 1, locale: 'en', colorScheme: 'system', textScale: 1,
        platform: 'web', restoredSnapshot: null, ...context,
      } });
      const handle = created.handle;
      let update = created.update;
      let sequence = 1;
      let disposed = false;
      const alive = () => { if (disposed) throw new Error('Mosaic host is disposed'); };
      return {
        get update() { alive(); return update; },
        dispatch(name, payload = {}) {
          alive();
          if (!Number.isSafeInteger(sequence)) throw new Error('Mosaic event sequence exhausted');
          const next = request({ op: 'dispatch', handle,
            event: { protocolVersion: 1, sequence, name, payload } });
          sequence += 1; // rejected events do not consume their sequence
          update = next;
          return update;
        },
        snapshot() { alive(); return request({ op: 'snapshot', handle }); },
        restore(snapshot) {
          alive();
          update = request({ op: 'restore', handle, snapshot });
          return update;
        },
        dispose() {
          if (disposed) return;
          request({ op: 'destroy', handle });
          disposed = true;
        },
      };
    },
  };
}
