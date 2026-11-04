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
  await t.test('invalid profile keeps valid agents usable and source aliases visible', async () => {
    const p=await page('window.invalidProfile=true'); await p.click('[data-target="codex"]');
    assert.equal(await p.evaluate('document.querySelector("[data-action=preview-distribution]").disabled'),false);
    await p.click('[data-page="agents"]');assert.ok((await p.text()).includes('损坏档案'));await p.click('[data-agent="codex"]');
    await until(()=>p.evaluate('document.body.innerText.includes("/fixture/source-link")'),'alias');
    assert.ok((await p.text()).includes('/fixture/source/code-review'));
  });
  await t.test('profile disable explains retained files and clears old pairs', async () => {
    const p=await page();await p.click('[data-target="codex"]');await p.click('[data-page="agents"]');await p.click('[data-agent="codex"]');
    await p.evaluate('[...document.querySelectorAll("button")].find(b=>b.textContent==="停用档案").click()');
    assert.ok((await p.text()).includes('不会卸载或删除'));await p.click('[data-action="confirm-disable"]');
    await until(()=>p.evaluate('window.calls.some(([c])=>c==="set_agent_profile_enabled")'),'disable');
    await until(()=>p.evaluate('!document.querySelector("dialog[open]")'),'disable complete');await p.click('[data-page="library"]');
    assert.equal(await p.evaluate('document.querySelector("[data-target=codex]").disabled'),true);
    assert.equal(await p.evaluate('document.querySelector("[data-action=preview-distribution]").disabled'),true);
  });
  await t.test('unsaved edits and settings require discard confirmation', async () => {
    const p=await page('window.acceptDiscard=false');await p.click('[data-action="edit-skill"]');
    await until(()=>p.evaluate('!!document.querySelector("textarea[name=document]")'),'editor');await p.fill('textarea[name=document]','unsaved');await p.click('dialog button[aria-label="关闭"]');
    assert.ok(await p.evaluate('!!document.querySelector("dialog[open]")'));await p.evaluate('window.acceptDiscard=true');await p.click('dialog button[aria-label="关闭"]');
    await p.click('[data-page="settings"]');await p.fill('input[aria-label="配置库目录"]','/fixture/another-library');await p.evaluate('window.acceptDiscard=false');await p.click('[data-page="history"]');
    assert.ok(await p.evaluate('!!document.querySelector("[data-action=save-settings]")'));await p.evaluate('window.acceptDiscard=true');await p.click('[data-page="library"]');
  });
  await t.test('settings library switch clears pairs without automatic apply', async () => {
    const p=await page();await p.click('[data-target="codex"]');await p.click('[data-page="settings"]');await p.fill('input[aria-label="配置库目录"]','/fixture/another-library');await p.click('[data-action="save-settings"]');
    await until(()=>p.evaluate('window.calls.filter(([c])=>c==="set_context").length>=2'),'new library');await p.click('[data-page="library"]');
    assert.equal(await p.evaluate('document.querySelector("[data-action=preview-distribution]").disabled'),true);assert.equal(await p.evaluate('window.calls.some(([c])=>c==="commit_distribution")'),false);
  });
  await t.test('custom shared reader is dynamic and cancellation never commits', async () => {
    const p=await page('window.fifth=true;window.customReader=true');await p.click('[data-target="codex"]');await p.click('[data-action="preview-distribution"]');
    await until(()=>p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'),'preview');assert.ok((await p.text()).includes('Fifth Agent（读取条件未知）'));
    await p.click('dialog button[aria-label="关闭"]');assert.equal(await p.evaluate('window.calls.some(([c])=>c==="commit_distribution")'),false);
    assert.ok(await p.evaluate('document.querySelector("[data-target=codex]").checked'));
  });
  await t.test('failed restore consumes preview and pending recovery survives reload', async () => {
    const p=await page('window.pendingRecovery=true;window.failRestore=true');assert.ok((await p.text()).includes('事务需要处理'));await p.click('[data-page="history"]');
    await p.click('[data-action="preview-recovery"]');await until(()=>p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'),'recovery preview');await p.click('[data-action="confirm-preview"]');
    await until(()=>p.evaluate('document.body.innerText.includes("本次确认已消费")'),'restore failure');assert.equal(await p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'),false);
    assert.equal(await p.evaluate('window.calls.filter(([c])=>c==="commit_restore").length'),1);await p.click('dialog button[aria-label="关闭"]');
    await p.call('Page.reload');await p.ready();assert.ok((await p.text()).includes('事务需要处理'));
  });
  await t.test('agent inventory reloads when scope changes without changing page',async()=>{
    const p=await page();await p.click('[data-page="agents"]');await p.click('[data-agent="codex"]');
    await until(()=>p.evaluate('document.body.innerText.includes("/fixture/source-link")'),'user inventory');await p.click('[data-scope="project"]');
    await until(()=>p.evaluate('document.body.innerText.includes("project-only-skill")'),'project inventory');
  });
  await t.test('no-op receipt never claims that files were applied',async()=>{
    const p=await page('window.noWrite=true');await p.click('[data-target="codex"]');await p.click('[data-action="preview-distribution"]');await until(()=>p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'),'preview');await p.click('[data-action="confirm-preview"]');
    await until(()=>p.evaluate('!!document.querySelector(".result-title")'),'receipt');assert.ok((await p.text()).includes('没有写入内容'));assert.equal((await p.text()).includes('文件已应用'),false);
  });
  await t.test('native async confirm never discards edited text without an app decision',async()=>{
    const p=await page('window.confirm=async()=>false');await p.click('[data-action="edit-skill"]');await until(()=>p.evaluate('!!document.querySelector("textarea[name=document]")'),'editor');await p.fill('textarea[name=document]','keep native draft');await p.click('dialog button[aria-label="关闭"]');
    await p.click('[data-action="discard-cancel"]');assert.equal(await p.evaluate('document.querySelector("textarea[name=document]")?.value'),'keep native draft');
  });
  await t.test('resolved failed edit receipts preserve drafts and show every outcome',async()=>{
    for(const outcome of ['failed_restored','recovery_required','audit_failed']){
      const p=await page(`window.failReceipt="${outcome}"`);await p.click('[data-action="edit-skill"]');await until(()=>p.evaluate('!!document.querySelector("textarea[name=document]")'),'editor');await p.fill('textarea[name=document]','keep receipt draft');await p.click('[data-action="save-document"]');
      await until(()=>p.evaluate('!!document.querySelector("[data-mutation-receipt]")'),'failed receipt');assert.equal(await p.evaluate('document.querySelector("textarea[name=document]")?.value'),'keep receipt draft');assert.ok((await p.text()).includes('/fixture/first'));assert.ok((await p.text()).includes('/fixture/failed'));
    }
  });
  await t.test('resolved failed profile save and disable never close as success',async()=>{
    const p=await page('window.failReceipt="failed_restored"');await p.click('[data-page="agents"]');await p.click('[data-action="add-profile"]');await p.click('[data-action="preview-profile"]');await p.click('[data-action="acknowledge-roots"]');await p.click('[data-action="commit-profile"]');
    await until(()=>p.evaluate('!!document.querySelector("[data-mutation-receipt]")'),'profile failure');assert.ok((await p.text()).includes('添加 Agent'));await p.click('dialog button[aria-label="关闭"]');await p.click('[data-agent="codex"]');await p.evaluate('[...document.querySelectorAll("button")].find(b=>b.textContent==="停用档案").click()');await p.click('[data-action="confirm-disable"]');
    await until(()=>p.evaluate('!!document.querySelector("[data-mutation-receipt]")'),'disable failure');assert.ok((await p.text()).includes('停用 Agent 档案'));
  });
  await t.test('corrected edit and settings can retry using fresh matching baselines',async()=>{
    const p=await page('window.retryEdit=true;window.retrySettings=true');await p.click('[data-action="edit-skill"]');await until(()=>p.evaluate('!!document.querySelector("textarea[name=document]")'),'editor');await p.fill('textarea[name=document]','invalid first draft');await p.click('[data-action="save-document"]');await until(()=>p.evaluate('!!document.querySelector("[data-editor-error]")'),'validation error');
    await p.fill('textarea[name=document]','---\nname: code-review\ndescription: Corrected.\n---\nCorrected draft.');await p.click('[data-action="save-document"]');await until(()=>p.evaluate('!document.querySelector("dialog[open]")'),'corrected edit saved');
    await p.click('[data-page="settings"]');await p.fill('input[aria-label="配置库目录"]','/fixture/overlap');await p.click('[data-action="save-settings"]');await until(()=>p.evaluate('document.querySelector(".settings-panel [role=alert]")'),'settings validation error');await p.fill('input[aria-label="配置库目录"]','/fixture/corrected-library');await p.click('[data-action="save-settings"]');await until(()=>p.evaluate('window.settingsSaved===true'),'corrected settings saved');
  });
  await t.test('retry refuses an externally changed edit or settings baseline',async()=>{
    for(const kind of ['edit','settings']){
      const p=await page(`window.retry${kind==='edit'?'Edit':'Settings'}=true`);
      if(kind==='edit'){
        await p.click('[data-action="edit-skill"]');await until(()=>p.evaluate('!!document.querySelector("textarea[name=document]")'),'editor');await p.fill('textarea[name=document]','keep changed baseline draft');await p.click('[data-action="save-document"]');await until(()=>p.evaluate('!!document.querySelector("[data-editor-error]")'),'first error');await p.evaluate('window.externalEdit=true');await p.click('[data-action="save-document"]');
        await until(()=>p.evaluate('!document.querySelector("[data-action=save-document]").disabled'),'retry complete');assert.equal(await p.evaluate('window.calls.filter(([c])=>c==="save_skill_document").length'),1);assert.equal(await p.evaluate('document.querySelector("textarea[name=document]").value'),'keep changed baseline draft');
      }else{
        await p.click('[data-page="settings"]');await p.fill('input[aria-label="配置库目录"]','/fixture/changed');await p.click('[data-action="save-settings"]');await until(()=>p.evaluate('!!document.querySelector(".settings-panel [role=alert]")'),'first error');await p.evaluate('window.externalSettings=true');await p.click('[data-action="save-settings"]');await until(()=>p.evaluate('!document.querySelector("[data-action=save-settings]").disabled'),'retry complete');assert.equal(await p.evaluate('window.calls.filter(([c])=>c==="save_settings").length'),1);
      }
    }
  });
  await t.test('invalidated profile preview is cancelled by its exact ID',async()=>{
    const p=await page();await p.click('[data-page="agents"]');await p.click('[data-action="add-profile"]');await p.click('[data-action="preview-profile"]');await until(()=>p.evaluate('!!document.querySelector("[data-action=commit-profile]")'),'profile preview');await p.fill('dialog input[value="Example Agent"]','Changed Agent');
    await until(()=>p.evaluate('window.calls.some(([c,a])=>c==="cancel_preview"&&a.previewId==="profile-preview")'),'profile cancellation');
  });
  await t.test('binary removals, empty directories and permission-only changes are explicit',async()=>{
    const p=await page('window.binaryRemoval=true');await p.click('[data-target="codex"]');await p.click('[data-action="preview-distribution"]');await until(()=>p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'),'preview');
    const text=await p.text();assert.ok(text.includes('old-image.bin'));assert.ok(text.includes('移出加载目录'));assert.ok(text.includes('仅改变执行权限'));assert.ok(text.includes('目录'));
  });
  await t.test('failed project reads cannot reuse user-scope status or installation evidence',async()=>{
    const p=await page('window.failProjectRead=true');await until(()=>p.evaluate('document.body.innerText.includes("内容一致")'),'initial status');await p.click('[data-scope="project"]');await until(()=>p.evaluate('document.body.innerText.includes("无法读取目标状态")'),'project read failure');
    assert.equal((await p.text()).includes('内容一致'),false);await p.click('[data-page="agents"]');await until(()=>p.evaluate('document.body.innerText.includes("无法检查安装状态")'),'detection failure');assert.equal((await p.text()).includes('已检测到 Agent'),false);
  });
  await t.test('user scope does not retain a removed project root',async()=>{
    const p=await page();await p.click('[data-scope="project"]');await until(()=>p.evaluate('document.querySelector("[data-scope=project]").getAttribute("aria-pressed")==="true"'),'project');await p.evaluate('window.removedProject=true');await p.click('[data-scope="user"]');
    await until(()=>p.evaluate('document.querySelector("[data-scope=user]").getAttribute("aria-pressed")==="true"'),'user scope');assert.equal(await p.evaluate('window.calls.filter(([c])=>c==="set_context").at(-1)[1].projectRoot'),null);
  });
  await t.test('viewing a profile does not permanently break disable',async()=>{
    const p=await page('window.staleDisable=true');await p.click('[data-page="agents"]');await p.click('[data-agent="codex"]');await p.evaluate('[...document.querySelectorAll("button")].find(b=>b.textContent==="查看 / 配置目录").click()');await until(()=>p.evaluate('window.profileRead===true'),'profile read');await p.click('dialog button[aria-label="关闭"]');await p.evaluate('[...document.querySelectorAll("button")].find(b=>b.textContent==="停用档案").click()');await p.click('[data-action="confirm-disable"]');await until(()=>p.evaluate('!document.querySelector("dialog[open]")'),'disable with fresh credential');
  });
  await t.test('profile enabled toggle is an unsaved change',async()=>{
    const p=await page();await p.click('[data-page="agents"]');await p.click('[data-action="add-profile"]');await p.click('dialog .switch-label input');await p.click('dialog button[aria-label="关闭"]');await p.click('[data-action="discard-cancel"]');assert.ok(await p.evaluate('!!document.querySelector("[data-action=preview-profile]")'));
  });
  await t.test('four page and task screenshots at desktop and narrow widths',async()=>{
    const p=await page();await mkdir('artifacts',{recursive:true});await p.evaluate('document.fonts.ready');
    for(const width of [1024,320]){
      await p.call('Emulation.setDeviceMetricsOverride',{width,height:900,deviceScaleFactor:1,mobile:false});
      for(const page of ['agents','history','settings']){
        await p.click(`[data-page="${page}"]`);await until(()=>p.evaluate('!document.querySelector(".loading")'),'page data');
        assert.ok(await p.evaluate('document.documentElement.scrollWidth<=window.innerWidth'),`${page} overflow ${width}`);
        const shot=await p.call('Page.captureScreenshot',{format:'png'});await writeFile(`artifacts/${page}-${width}.png`,Buffer.from(shot.data,'base64'));
      }
      await p.click('[data-page="library"]');await p.click('[data-target="codex"]');await p.click('[data-action="preview-distribution"]');await until(()=>p.evaluate('!!document.querySelector("[data-action=confirm-preview]")'),'preview');
      const shot=await p.call('Page.captureScreenshot',{format:'png'});await writeFile(`artifacts/preview-${width}.png`,Buffer.from(shot.data,'base64'));await p.click('dialog button[aria-label="关闭"]');await p.click('[data-target="codex"]');
    }
  });
  await t.test('responsive screenshots, dark theme and dialog keyboard focus', async () => {
    const p = await page(); await mkdir('artifacts', { recursive: true }); await p.evaluate('document.fonts.ready');
    for (const width of [1024, 850, 650, 390, 320]) {
      await p.call('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
      assert.ok(await p.evaluate('document.documentElement.scrollWidth<=window.innerWidth'), `overflow at ${width}`);
      const shot = await p.call('Page.captureScreenshot', { format: 'png' }); await writeFile(`artifacts/library-${width}.png`, Buffer.from(shot.data, 'base64'));
    }
    await p.call('Emulation.setDeviceMetricsOverride', { width: 1024, height: 900, deviceScaleFactor: 1, mobile: false }); await p.click('[data-action="theme"]');
    await p.evaluate('Promise.all(document.getAnimations().map(a=>a.finished.catch(()=>{})))');
    const dark = await p.call('Page.captureScreenshot', { format: 'png' }); await writeFile('artifacts/library-dark.png', Buffer.from(dark.data, 'base64'));
    await p.click('[data-action="edit-skill"]'); await until(() => p.evaluate('!!document.querySelector("dialog[open]")'), 'dialog');
    await p.call('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27, nativeVirtualKeyCode: 27 }); await p.call('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27, nativeVirtualKeyCode: 27 });
    await until(() => p.evaluate('!document.querySelector("dialog[open]")'), 'dialog close'); assert.equal(await p.evaluate('document.activeElement.dataset.action'), 'edit-skill');
  });
});
