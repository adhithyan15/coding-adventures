import { test, expect } from '@playwright/test';
import { readFile, writeFile } from 'node:fs/promises';

for (const colorScheme of ['light', 'dark']) {
  test(`downloaded bundle edits and reopens saved bytes (${colorScheme})`, async ({ browser }, testInfo) => {
    const context = await browser.newContext({ colorScheme, viewport: { width: 1280, height: 720 } });
    const savedPath = testInfo.outputPath('Workbook.visicalc');
    // Simulate only OS picker handles. The production app, Rust serializer,
    // file capability executor and generated controls are unmodified.
    await context.exposeBinding('releaseWrite', async (_, bytes) => writeFile(savedPath, Buffer.from(bytes)));
    await context.exposeBinding('releaseRead', async () => [...await readFile(savedPath)]);
    await context.addInitScript(() => {
      window.showSaveFilePicker = async () => ({
        name: 'Workbook.visicalc',
        createWritable: async () => ({
          write: async bytes => window.releaseWrite([...bytes]), close: async () => {}, abort: async () => {},
        }),
      });
      window.showOpenFilePicker = async () => [{
        name: 'Workbook.visicalc',
        getFile: async () => new File([new Uint8Array(await window.releaseRead())], 'Workbook.visicalc'),
      }];
    });
    try {
      const page = await context.newPage();
      const errors = [];
      page.on('pageerror', error => errors.push(error.message));
      await page.goto(process.env.VISICALC_URL || 'http://127.0.0.1:8080');
      const formula = page.getByRole('textbox', { name: 'Enter a value or formula' });
      await expect(formula).toHaveValue('15');
      await formula.fill('=2+3'); await formula.press('Enter');
      await expect(page.getByRole('row', { name: '1 5 3 12 8 28', exact: true })).toBeVisible();
      await page.getByRole('button', { name: 'Save', exact: true }).click();
      await expect(page.getByRole('status')).toHaveText('Saved Workbook.visicalc');
      expect((await readFile(savedPath)).length).toBeGreaterThan(0);
      await page.close();

      // A fresh page creates a fresh Rust app. Only the saved file crosses over.
      const reopened = await context.newPage();
      reopened.on('pageerror', error => errors.push(error.message));
      await reopened.goto(process.env.VISICALC_URL || 'http://127.0.0.1:8080');
      await expect(reopened.getByRole('textbox', { name: 'Enter a value or formula' })).toHaveValue('15');
      await reopened.getByRole('button', { name: 'Open', exact: true }).click();
      await expect(reopened.getByRole('status')).toHaveText('Opened Workbook.visicalc');
      await expect(reopened.getByRole('textbox', { name: 'Enter a value or formula' })).toHaveValue('=2+3');
      await expect(reopened.getByRole('row', { name: '1 5 3 12 8 28', exact: true })).toBeVisible();
      const table = reopened.getByRole('table', { name: 'Data table' });
      for (let index = 0; index < 15; index++) await table.press('ArrowDown');
      await table.press('F2');
      const editor = reopened.getByRole('textbox', { name: 'Cell A16', exact: true });
      await expect(editor).toBeVisible(); await expect(editor).toBeFocused();
      await editor.fill('42'); await editor.press('Enter');
      await expect(table).toBeFocused();
      await expect(reopened.getByRole('status')).toContainText('Updated A16, 42');
      await reopened.screenshot({ path: testInfo.outputPath(`workbook-${colorScheme}.png`) });
      expect(errors).toEqual([]);
    } finally { await context.close(); }
  });
}
