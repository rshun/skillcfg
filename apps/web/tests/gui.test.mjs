import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mkdir, writeFile } from 'node:fs/promises';
import { browser, until } from './browser.mjs';
import { installFixture } from './fixture.mjs';
test('approved workbench browser interactions', { timeout: 240000 }, async t => {
  const b = await browser(t);
  async function page(setup = '') { const p = await b.page(installFixture, setup); await p.ready(); return p; }
  await t.test('four navigation destinations and dynamic fifth agent', async () => {
    const p = await page('window.fifth=true');
    assert.equal(await p.evaluate('document.querySelectorAll(".nav-item").length'), 4);
    await p.click('[data-skill="package-one"]'); assert.ok((await p.text()).includes('Fifth Agent'));
  });
  await t.test('import_never_applies', async () => {
    const p = await page(); await p.click('[data-action="import-local"]'); await p.click('[data-action="pick-import"]'); await p.click('[data-action="scan-import"]');
    await until(() => p.evaluate('!!document.querySelector("[data-import-item]")'), 'import scan'); await p.click('[data-import-item]'); await p.click('[data-action="preview-import"]');
    await until(() => p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'), 'import preview'); await p.click('[data-action="confirm-preview"]');
    await until(() => p.evaluate('window.calls.some(([c])=>c==="commit_import")'), 'save import');
    assert.equal(await p.evaluate('window.calls.some(([c])=>c==="commit_distribution")'), false);
  });
  await t.test('editor_keeps_dirty_input_on_error', async () => {
    const p = await page('window.failSave=true'); await p.click('[data-action="edit-skill"]');
    await until(() => p.evaluate('!!document.querySelector("textarea[name=document]")'), 'editor');
    await p.fill('textarea[name=document]', '---\nname: kept\ndescription: Keep input.\n---\nEdited.'); await p.click('[data-action="save-document"]');
    await until(() => p.evaluate('!!document.querySelector("[data-editor-error]")'), 'save failure');
    assert.ok(await p.evaluate('document.querySelector("textarea[name=document]").value.includes("Edited.")'));
    assert.equal(await p.evaluate('window.calls.some(([c])=>c==="commit_distribution")'), false);
  });
  await t.test('cancel_project_picker_keeps_context', async () => {
    const p = await page('window.cancelPicker=true'); await p.click('[data-scope="project"]');
    await until(() => p.evaluate('window.calls.some(([c])=>c==="choose_directory")'), 'picker');
    assert.equal(await p.evaluate('document.querySelector("[data-scope=user]").getAttribute("aria-pressed")'), 'true');
  });
  await t.test('shared_target_lists_extra_readers and full old-file archive effect', async () => {
    const p = await page(); await p.click('[data-target="codex"]'); await p.click('[data-action="preview-distribution"]');
    await until(() => p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'), 'preview');
    const text = await p.text(); assert.ok(text.includes('可能额外读取')); assert.ok(text.includes('Antigravity')); assert.ok(text.includes('old-extra.txt')); assert.ok(text.includes('归档'));
  });
  await t.test('stale_preview_has_no_confirm_button', async () => {
    const p = await page('window.expired=true'); await p.click('[data-target="codex"]'); await p.click('[data-action="preview-distribution"]');
    await until(() => p.evaluate('document.body.innerText.includes("预览已过期")'), 'expiry'); assert.equal(await p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'), false);
  });
  await t.test('double_confirm_is_not_replayed and partial_failure_retains_successful_groups', async () => {
    const p = await page('window.slowCommit=true;window.partial=true'); await p.click('[data-target="codex"]'); await p.click('[data-action="preview-distribution"]');
    await until(() => p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'), 'ready');
    await p.evaluate('document.querySelector("[data-action=confirm-preview]").click();document.querySelector("[data-action=confirm-preview]")?.click()');
    await until(() => p.evaluate('typeof window.releaseCommit==="function"'), 'commit pending'); assert.equal(await p.evaluate('window.calls.filter(([c])=>c==="commit_distribution").length'), 1);
    await p.evaluate('window.releaseCommit()'); await until(() => p.evaluate('document.body.innerText.includes("失败，已恢复")'), 'partial receipt'); assert.ok((await p.text()).includes('已写入'));
  });
  await t.test('restore_has_independent_preview', async () => {
    const p = await page(); await p.click('[data-page="history"]'); await until(() => p.evaluate('!!document.querySelector("[data-action=preview-restore]")'), 'history'); await p.click('[data-action="preview-restore"]');
    await until(() => p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'), 'restore preview'); await p.click('[data-action="confirm-preview"]');
    await until(() => p.evaluate('window.calls.some(([c])=>c==="commit_restore")'), 'restore'); assert.equal(await p.evaluate('window.calls.some(([c])=>c==="commit_distribution")'), false);
  });
  await t.test('add_custom_agent_requires_root_confirmation', async () => {
    const p = await page(); await p.click('[data-page="agents"]'); await p.click('[data-action="add-profile"]'); await p.click('[data-action="preview-profile"]');
    await until(() => p.evaluate('!!document.querySelector("[data-action=commit-profile]")'), 'profile preview');
    assert.equal(await p.evaluate('document.querySelector("[data-action=commit-profile]").disabled'), true); assert.ok((await p.text()).includes('/fixture/.fifth/skills'));
    await p.click('[data-action="acknowledge-roots"]'); await p.click('[data-action="commit-profile"]');
    await until(() => p.evaluate('document.body.innerText.includes("Fifth Agent")'), 'dynamic agent');
  });
  await t.test('slow_old_agent_response_is_ignored', async () => {
    const p = await page('window.slowInventory=true'); await p.click('[data-page="agents"]'); await p.click('[data-agent="codex"]');
    await until(() => p.evaluate('typeof window.releaseInventory==="function"'), 'slow inventory'); await p.click('[data-page="library"]'); await p.evaluate('window.releaseInventory()');
    assert.equal((await p.text()).includes('old-response'), false);
  });
  await t.test('corrupt_library_is_not_an_empty_editable_library', async () => {
    const p = await page('window.badLibrary=true'); assert.ok((await p.text()).includes('无法读取配置库')); assert.equal(await p.evaluate('!!document.querySelector("[data-action=edit-skill]")'), false);
  });
  await t.test('responsive screenshots, dark theme and dialog keyboard focus', async () => {
    const p = await page(); await mkdir('artifacts', { recursive: true });
    for (const width of [1024, 850, 650, 390, 320]) {
      await p.call('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
      assert.ok(await p.evaluate('document.documentElement.scrollWidth<=window.innerWidth'), `overflow at ${width}`);
      const shot = await p.call('Page.captureScreenshot', { format: 'png' }); await writeFile(`artifacts/library-${width}.png`, Buffer.from(shot.data, 'base64'));
    }
    await p.call('Emulation.setDeviceMetricsOverride', { width: 1024, height: 900, deviceScaleFactor: 1, mobile: false }); await p.click('[data-action="theme"]');
    const dark = await p.call('Page.captureScreenshot', { format: 'png' }); await writeFile('artifacts/library-dark.png', Buffer.from(dark.data, 'base64'));
    await p.click('[data-action="edit-skill"]'); await until(() => p.evaluate('!!document.querySelector("dialog[open]")'), 'dialog');
    await p.call('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Escape', code: 'Escape' }); await p.call('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape' });
    await until(() => p.evaluate('!document.querySelector("dialog[open]")'), 'dialog close'); assert.equal(await p.evaluate('document.activeElement.dataset.action'), 'edit-skill');
  });
});
