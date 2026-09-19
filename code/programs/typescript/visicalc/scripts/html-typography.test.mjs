import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { JSDOM } from 'jsdom';

test('generated HTML hydrates typed font sizes and restores static fallback', async () => {
  const path = process.env.MOSAIC_HTML_TYPOGRAPHY_OUTPUT;
  assert.ok(path, 'generate the Rust typography fixture first');
  const dom = new JSDOM(readFileSync(join(path, 'index.html'), 'utf8'), { runScripts: 'outside-only' });
  const { window } = dom;
  let size = 27;
  const events = [];
  window.mosaicHost = {
    getProps: () => ({textSize: size, rows: ['Loop label']}),
    handleEvent: ({event}) => { events.push(event); },
  };
  window.eval(readFileSync(join(path, 'main.js'), 'utf8'));
  const flush = () => new Promise(resolve => setTimeout(resolve, 0));
  await flush();
  for (size of [18, 27, 36, 0, -1, NaN, Infinity, '24', '12px;color:red', null, 24]) {
    window.dispatchEvent(new window.Event('mosaic-host-ready'));
    await flush();
    const nodes = [...window.document.querySelectorAll('[data-mosaic-font-size-slot]')];
    assert.equal(nodes.length, 10);
    const valid = typeof size === 'number' && Number.isFinite(size) && size > 0;
    const defaults = ['18px', '16px', '15px', '14px', '', ...Array(5).fill('18px')];
    nodes.forEach((node, i) => assert.equal(node.style.fontSize, valid ? `${size}px` : defaults[i]));
    assert.equal(nodes[0].style.fontFamily, 'monospace');
    assert.equal(nodes[0].style.color, 'rgb(18, 52, 86)');
    assert.equal(nodes[4].textContent, 'Loop label');
    const cells = [...window.document.querySelectorAll('th, td')];
    assert.deepEqual(cells.map(cell => cell.textContent), ['Header', 'Body', 'Loop label', 'Loop label', 'Static', 'Footer']);
    for (const cell of cells) {
      assert.equal(cell.style.fontSize, valid && cell.textContent !== 'Static' ? `${size}px` : '18px');
      assert.equal(cell.style.fontFamily, 'monospace');
      assert.equal(cell.style.color, 'rgb(18, 52, 86)');
    }
  }
  window.document.querySelector('button').click();
  await flush();
  assert.equal(events.length, 1);
  window.close();
});
