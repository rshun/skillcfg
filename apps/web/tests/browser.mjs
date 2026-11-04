import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as pause } from 'node:timers/promises';
export async function until(read, label, timeout = 7000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) { const value = await read(); if (value) return value; await pause(50); }
  throw new Error(`Timed out: ${label}`);
}
export async function browser(t) {
  const server = spawn(process.execPath, ['node_modules/vite/bin/vite.js', 'preview', '--host', '127.0.0.1', '--port', '4173', '--strictPort'], { stdio: 'ignore' });
  t.after(() => server.kill());
  await until(async () => { try { return (await fetch('http://127.0.0.1:4173')).ok; } catch { return false; } }, 'preview server');
  const profile = await mkdtemp(join(tmpdir(), 'skill-hub-gui-'));
  const chrome = spawn(process.env.CHROME_PATH || '/usr/bin/google-chrome', ['--headless', '--no-sandbox', '--disable-dev-shm-usage', '--remote-debugging-pipe', `--user-data-dir=${profile}`], { stdio: ['ignore', 'ignore', 'pipe', 'pipe', 'pipe'] });
  t.after(() => chrome.kill()); await once(chrome, 'spawn');
  let seq = 0, buffer = ''; const pending = new Map();
  chrome.stderr.resume();
  chrome.on('exit', () => { for (const p of pending.values()) { clearTimeout(p.timer); p.reject(new Error('Test Chrome exited')); } pending.clear(); });
  chrome.stdio[4].setEncoding('utf8');
  chrome.stdio[4].on('data', chunk => {
    buffer += chunk; let end;
    while ((end = buffer.indexOf('\0')) >= 0) {
      const message = JSON.parse(buffer.slice(0, end)); buffer = buffer.slice(end + 1);
      const p = pending.get(message.id); if (!p) continue; pending.delete(message.id); clearTimeout(p.timer);
      if (message.error) p.reject(new Error(message.error.message)); else p.resolve(message.result);
    }
  });
  function send(method, params = {}, sessionId) {
    return new Promise((resolve, reject) => {
      const id = ++seq, timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 20000);
      pending.set(id, { resolve, reject, timer }); chrome.stdio[3].write(JSON.stringify({ id, method, params, sessionId }) + '\0');
    });
  }
  await send('Browser.getVersion');
  return { async page(fixture, setup = '') {
    const { targetId } = await send('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await send('Target.attachToTarget', { targetId, flatten: true });
    const call = (m, p = {}) => send(m, p, sessionId);
    await call('Page.enable');
    await call('Page.bringToFront');
    await call('Page.addScriptToEvaluateOnNewDocument', { source: `(${fixture.toString()})();${setup}` });
    const evaluate = async expression => {
      const r = await call('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.text || 'Browser expression failed'); return r.result.value;
    };
    await call('Page.navigate', { url: 'http://127.0.0.1:4173' });
    await until(() => evaluate('document.querySelector("#root")?.textContent?.includes("Skill Hub")'), 'React ready');
    return { call, evaluate, click: async selector => {
      await until(()=>evaluate(`!!document.querySelector(${JSON.stringify(selector)})`),`control: ${selector}`);
      await evaluate(`(() => { const el=document.querySelector(${JSON.stringify(selector)});el.focus();el.click(); })()`);
    }, fill: (selector, value) => evaluate(`(() => { const el=document.querySelector(${JSON.stringify(selector)}); if (!el) throw new Error('Missing input'); Object.getOwnPropertyDescriptor(el.tagName==='TEXTAREA'?HTMLTextAreaElement.prototype:HTMLInputElement.prototype,'value').set.call(el,${JSON.stringify(value)}); el.dispatchEvent(new Event('input',{bubbles:true})); })()`),
    text: () => evaluate('document.body.innerText'), ready: () => until(() => evaluate('!!document.querySelector("[data-ready=true]")'), 'workbench loaded') };
  } };
}
