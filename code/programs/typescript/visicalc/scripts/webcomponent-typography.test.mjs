import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { JSDOM } from 'jsdom';

test('generated custom element hydrates numeric font sizes and restores static fallback', async () => {
  const path = process.env.MOSAIC_WC_TYPOGRAPHY_OUTPUT;
  assert.ok(path, 'generate the Rust typography fixture first');
  const dom = new JSDOM('<!doctype html><body></body>', { runScripts: 'dangerously' });
  const { window } = dom;
  window.eval(readFileSync(join(path, 'Typography.js'), 'utf8'));
  const root = window.document.createElement('mos-typography');
  root.setAttribute('rows', JSON.stringify(['Loop label']));
  window.document.body.append(root);
  let size;
  const events = [];
  root.addEventListener('mosaic:action', event => events.push(event.detail));
  const flush = () => new Promise(resolve => setTimeout(resolve, 0));
  for (size of [18, 27, 36, 0, -1, NaN, Infinity, '24', '12px;color:red', null, 24]) {
    if (size === null) root.removeAttribute('text-size');
    else root.setAttribute('text-size', String(size));
    await flush();
    const nodes = [...root.shadowRoot.querySelectorAll('[data-mosaic-font-size-slot]')];
    assert.equal(nodes.length, 10);
    const valid = size !== null && Number.isFinite(Number(size)) && Number(size) > 0;
    const defaults = ['18px', '16px', '15px', '14px', '', ...Array(5).fill('18px')];
    nodes.forEach((node, i) => assert.equal(node.style.fontSize, valid ? `${size}px` : defaults[i]));
    assert.equal(nodes[0].style.fontFamily, 'monospace');
    assert.equal(nodes[0].style.color, 'rgb(18, 52, 86)');
    assert.equal(nodes[4].textContent, 'Loop label');
    const cells = [...root.shadowRoot.querySelectorAll('th, td')];
    assert.deepEqual(cells.map(cell => cell.textContent), ['Header', 'Body', 'Loop label', 'Loop label', 'Static', 'Footer']);
    for (const cell of cells) {
      assert.equal(cell.style.fontSize, valid && cell.textContent !== 'Static' ? `${size}px` : '18px');
      assert.equal(cell.style.fontFamily, 'monospace');
      assert.equal(cell.style.color, 'rgb(18, 52, 86)');
    }
  }
  root.shadowRoot.querySelector('button').click();
  await flush();
  assert.equal(events.length, 1);
  window.close();
});


test('generated custom element scopes table fonts and preserves authored overrides', async () => {
  const path = process.env.MOSAIC_WC_TYPOGRAPHY_OUTPUT;
  const dom = new JSDOM('<!doctype html><body></body>', { runScripts: 'dangerously' });
  const { window } = dom;
  window.eval(readFileSync(join(path, 'table/Tables.js'), 'utf8'));
  const root = window.document.createElement('mos-tables');
  root.setAttribute('rows', JSON.stringify(['First']));
  window.document.body.append(root);
  const actions = [];
  root.addEventListener('mosaic:action', event => actions.push(event.detail));
  let size = 18;
  let rows;
  const flush = () => new Promise(resolve => setTimeout(resolve, 0));
  await flush();
  for (size of [18, 27, 36, 0, -1, NaN, Infinity, '24', 24]) {
    rows = ['First', 'New row'];
    root.setAttribute('rows', JSON.stringify(rows));
    root.setAttribute('size', String(size));
    await flush();
    const tables = [...root.shadowRoot.querySelectorAll('table')];
    const valid = Number.isFinite(Number(size)) && Number(size) > 0;
    assert.equal(tables[0].style.fontSize, valid ? `${size}px` : '17px');
    assert.equal(tables[0].style.fontFamily, 'monospace');
    assert.equal(tables[1].style.fontSize, 'initial');
    const controls = [...root.shadowRoot.querySelectorAll('input, textarea, button')];
    assert.deepEqual(controls.map(e => e.style.fontSize), ['', ...Array(6).fill('inherit'), 'inherit', 'inherit', '25px', '']);
    const fixedRow = [...root.shadowRoot.querySelectorAll('tr')].find(e => e.textContent.includes('Fixed row'));
    assert.equal(fixedRow.style.fontSize, '21px');
    assert.ok([...root.shadowRoot.querySelectorAll('td')].some(e => e.textContent === 'New row'));
  }
  root.shadowRoot.querySelector('button').click();
  assert.equal(actions.length, 1);
  window.close();
});
