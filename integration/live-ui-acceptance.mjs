#!/usr/bin/env node
// Supervised, two-invocation browser acceptance. This program never changes service mode.
import { spawn, execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { promises as fs } from 'node:fs';
import os from 'node:os';
import assert from 'node:assert/strict';
import path from 'node:path';

const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
const fail = message => { throw new Error(message); };
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const stable = value => Array.isArray(value) ? value.map(stable) : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map(key => [key, stable(value[key])])) : value;
const same = (a, b) => JSON.stringify(stable(a)) === JSON.stringify(stable(b));
const expectedProfile = root => ({
  controls: { 'key_key.back': 's' }, crafting: { s: 'k' }, ninjabrain: {}, snippets: [],
  chat: { open_chat: 'enter', open_command: 'slash', send: 'enter', cancel: 'esc' },
  presets: [], launcher_root: root
});
const validPreview = (preview, instanceId) => preview.instances?.length === 1 &&
  preview.instances[0].instance_id === instanceId && preview.instances[0].status === 'ready' &&
  preview.mappings?.length === 1 &&
  same(preview.mappings[0], { instance_id: instanceId, physical: 's', emitted: 'k', action: 'key_key.back' });
const validApplyJob = (job, instanceId) => job.status === 'succeeded' &&
  typeof job.result?.backup_id === 'string' && /^[0-9a-f]{32}$/.test(job.result.backup_id) &&
  !job.result.keyd_error && job.result.instances?.length === 1 &&
  job.result.instances[0].instance_id === instanceId && job.result.instances[0].status === 'updated';
if (process.argv[2] === 'self-test') {
  assert.deepEqual(expectedProfile('/tmp/root').chat, { open_chat:'enter', open_command:'slash', send:'enter', cancel:'esc' });
  assert.equal(servePort(['/usr/local/bin/mcsr-toolkit', 'serve']), 8765);
  assert.equal(servePort(['/usr/local/bin/mcsr-toolkit', 'serve', '--enable-live-keyd', '--port', '8877']), 8877);
  for (const args of [['--port'], ['--port', '0'], ['--port', '65536'], ['--port', '8765', '--port', '8877'], ['--other']]) {
    assert.throws(() => servePort(['/usr/local/bin/mcsr-toolkit', 'serve', ...args]));
  }
  assert.deepEqual(listeningSocketInodes('sl local_address rem_address st tx_queue rx_queue tr tm->when retrnsmt uid timeout inode\n0: 0100007F:223D 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 12345\n1: 0100007F:223E 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 67890', 8765), new Set(['12345']));
  assert.equal(expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.s\r\nkey_key.chat:key.keyboard.enter\r\nkey_key.command:key.keyboard.slash\r\nkey_key.hotbar.7:key.keyboard.t\r\ngamma:1\r\n')).toString(), 'key_key.back:key.keyboard.k\r\nkey_key.chat:key.keyboard.enter\r\nkey_key.command:key.keyboard.slash\r\nkey_key.hotbar.7:key.keyboard.t\r\ngamma:1\r\n');
  assert.equal(expectedAppliedOptions(Buffer.from('gamma:1\rkey_key.chat:key.keyboard.enter\rkey_key.command:key.keyboard.slash\rkey_key.hotbar.7:key.keyboard.t\rkey_key.back:key.keyboard.s\r')).toString(), 'gamma:1\rkey_key.chat:key.keyboard.enter\rkey_key.command:key.keyboard.slash\rkey_key.hotbar.7:key.keyboard.t\rkey_key.back:key.keyboard.k\r');
  assert.equal(expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.s\nkey_key.chat:key.keyboard.enter\nkey_key.hotbar.7:key.keyboard.t\n')).toString(), 'key_key.back:key.keyboard.k\nkey_key.chat:key.keyboard.enter\nkey_key.hotbar.7:key.keyboard.t\n');
  assert.throws(() => expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.j\n')));
  assert.throws(() => expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.s\nkey_key.back:key.keyboard.s\n')));
  assert.throws(() => expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.s\nkey_key.chat:key.keyboard.t\nkey_key.hotbar.7:key.keyboard.t\n')));
  assert.throws(() => expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.s\nkey_key.hotbar.7:key.keyboard.t\n')));
  assert.throws(() => expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.s\nkey_key.chat:key.keyboard.enter\n')));
  assert.throws(() => expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.s\nkey_key.chat:key.keyboard.enter\nkey_key.chat:key.keyboard.enter\nkey_key.hotbar.7:key.keyboard.t\n')));
  assert.throws(() => expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.s\nkey_key.chat:key.keyboard.enter\nkey_key.hotbar.7:key.keyboard.j\n')));
  assert.throws(() => expectedAppliedOptions(Buffer.from('key_key.back:key.keyboard.s\nkey_key.chat:key.keyboard.enter\nkey_key.command:key.keyboard.t\nkey_key.hotbar.7:key.keyboard.t\n')));
  assert.deepEqual(expectedKeydSnapshot(), [
    ['mcsr-toolkit', sha('[mcsr_chat]\ns = s\nslash = slash\nenter = clearm(enter)\nesc = clearm(esc)\n')],
    ['mcsr-toolkit.conf', sha('# MCSR Toolkit managed\n[ids]\n*\n\ninclude mcsr-toolkit\n')]
  ]);
  const result = { status:'succeeded', result:{ backup_id:'0'.repeat(32), keyd_error:null, instances:[{ instance_id:'MCSR_Ranked', status:'updated', detail:'' }] } };
  assert.equal(validApplyJob(result, 'MCSR_Ranked'), true);
  assert.equal(validApplyJob({ ...result, result:{ ...result.result, instances:[{ instance_id:'MCSR_Ranked', status:'ready' }] } }, 'MCSR_Ranked'), false);
  assert.equal(validApplyJob(result, 'unexpected'), false);
  const preview = { instances:[{ instance_id:'MCSR_Ranked', status:'ready' }], mappings:[
    { instance_id:'MCSR_Ranked', physical:'s', emitted:'k', action:'key_key.back' }
  ] };
  assert.equal(validPreview(preview, 'MCSR_Ranked'), true);
  assert.equal(validPreview({ ...preview, instances:[{ instance_id:'Other', status:'ready' }] }, 'MCSR_Ranked'), false);
  assert.equal(validPreview({ ...preview, instances:[{ instance_id:'MCSR_Ranked', status:'failed' }] }, 'MCSR_Ranked'), false);
  assert.equal(validPreview({ ...preview, mappings:[...preview.mappings, { instance_id:'MCSR_Ranked', physical:'t', emitted:'t', action:'key_key.chat' }] }, 'MCSR_Ranked'), false);
  assert.equal(validPreview({ ...preview, mappings:[...preview.mappings, { instance_id:'MCSR_Ranked', physical:'t', emitted:'t', action:'key_key.hotbar.7' }] }, 'MCSR_Ranked'), false);
  assert.equal(validPreview({ ...preview, mappings:[] }, 'MCSR_Ranked'), false);
  assert.equal(validPreview({ ...preview, mappings:[...preview.mappings, preview.mappings[0]] }, 'MCSR_Ranked'), false);
  assert.equal(validPreview({ ...preview, mappings:[{ ...preview.mappings[0], physical:'t' }] }, 'MCSR_Ranked'), false);
  assert.equal(same(expectedProfile('/tmp/root'), { ...expectedProfile('/tmp/root'), snippets:['EXTRA'] }), false);
  const tempProfile = await fs.mkdtemp(path.join(os.tmpdir(), 'mcsr-cleanup-self-test-'));
  await fs.mkdir(path.join(tempProfile, 'Default'));
  const lockFile = path.join(tempProfile, 'Default', 'SingletonLock');
  await fs.writeFile(lockFile, 'locked');
  const fakeChrome = spawn(process.execPath, ['-e', `
    const fs = require('node:fs');
    process.stdout.write('ready\\n');
    process.on('SIGTERM', () => setTimeout(() => {
      try { fs.writeFileSync(${JSON.stringify(lockFile)}, 'closed'); }
      catch { process.exitCode = 1; }
      setTimeout(() => process.exit(), 50);
    }, 50));
    setInterval(() => {}, 1000);
  `], { stdio: ['ignore', 'pipe', 'ignore'] });
  try {
    await new Promise(resolve => fakeChrome.stdout.once('data', resolve));
    await closeTemporaryBrowser({ close() {} }, fakeChrome, tempProfile);
    assert.equal(fakeChrome.exitCode, 0, 'Chrome must exit before its profile is removed');
    await assert.rejects(fs.stat(tempProfile), { code:'ENOENT' });
  } finally {
    if (!childExited(fakeChrome)) { fakeChrome.kill('SIGKILL'); await waitForChildExit(fakeChrome, 5000); }
    await fs.rm(tempProfile, { recursive:true, force:true });
  }
  const missingProfile = await fs.mkdtemp(path.join(os.tmpdir(), 'mcsr-missing-chrome-self-test-'));
  const missingChrome = spawnChrome(path.join(missingProfile, 'missing-chrome'), missingProfile);
  try {
    const spawnError = await new Promise(resolve => missingChrome.once('error', resolve));
    assert.equal(spawnError.code, 'ENOENT');
    await closeTemporaryBrowser({ close() {} }, missingChrome, missingProfile);
    await assert.rejects(fs.stat(missingProfile), { code:'ENOENT' });
  } finally {
    await fs.rm(missingProfile, { recursive:true, force:true });
  }
  console.log('Apply result fixture passed');
  process.exit(0);
}
const phase = process.argv[2];
if (!['prepare', 'restore'].includes(phase)) fail('First argument must be prepare or restore');
const flags = new Map();
for (let i = 3; i < process.argv.length; i++) {
  const arg = process.argv[i];
  if (!arg.startsWith('--') || flags.has(arg)) fail(`Unknown or repeated argument: ${arg}`);
  if (['--confirm-live-prepare', '--confirm-live-restore'].includes(arg)) flags.set(arg, true);
  else flags.set(arg, process.argv[++i] ?? fail(`Missing value for ${arg}`));
}
const get = name => flags.get(`--${name}`) ?? fail(`--${name} is required`);
const allowed = new Set(['--state', '--url', '--launcher-root', '--instance-id', '--options-path', '--profile-json', '--profile-path', '--baseline-sha', '--chrome', '--timeout-ms', '--cleanup-profile', '--confirm-live-prepare', '--confirm-live-restore']);
for (const key of flags.keys()) if (!allowed.has(key)) fail(`Unknown option ${key}`);
if (!flags.get(`--confirm-live-${phase}`) || flags.get(`--confirm-live-${phase === 'prepare' ? 'restore' : 'prepare'}`)) fail(`Explicit --confirm-live-${phase} is required`);
const statePath = path.resolve(get('state'));
const baseUrl = new URL(get('url'));
const urlPort = /^http:\/\/127\.0\.0\.1:([0-9]{1,5})\/$/.exec(get('url'))?.[1];
if (!urlPort || Number(urlPort) < 1 || Number(urlPort) > 65535 ||
    baseUrl.hostname !== '127.0.0.1' || baseUrl.protocol !== 'http:' || baseUrl.pathname !== '/') fail('URL must be http://127.0.0.1:<port>/');
const root = path.resolve(get('launcher-root'));
const instanceId = get('instance-id');
const optionsPath = path.resolve(get('options-path'));
const profilePath = path.resolve(get('profile-path'));
const baselineSha = get('baseline-sha');
if (!/^[0-9a-f]{64}$/.test(baselineSha) || !/^[A-Za-z0-9._-]+$/.test(instanceId)) fail('Invalid baseline SHA or instance ID');
const timeoutMs = Number(flags.get('--timeout-ms') ?? 90000);
if (!Number.isInteger(timeoutMs) || timeoutMs < 10000 || timeoutMs > 600000) fail('Timeout must be 10000–600000 ms');
const run = (cmd, args) => execFileSync(cmd, args, { encoding: 'utf8', timeout: 10000 }).trim();
const hashFile = async p => sha(await fs.readFile(p));
const canonical = async p => fs.realpath(p);
function expectedAppliedOptions(baseline) {
  const text = new TextDecoder('utf-8', { fatal: true }).decode(baseline);
  const backward = text.split(/\r\n|\n|\r/).filter(line => line.startsWith('key_key.back:'));
  if (backward.length !== 1 || backward[0] !== 'key_key.back:key.keyboard.s') fail('Baseline Backward binding is not exactly S');
  for (const [binding, key] of [['chat', 'enter'], ['hotbar.7', 't']]) {
    const lines = text.split(/\r\n|\n|\r/).filter(line => line.startsWith(`key_key.${binding}:`));
    if (lines.length !== 1 || lines[0] !== `key_key.${binding}:key.keyboard.${key}`) fail(`Baseline ${binding} binding is not exactly ${key}`);
  }
  const command = text.split(/\r\n|\n|\r/).filter(line => line.startsWith('key_key.command:'));
  if (command.length > 1 || (command.length === 1 && command[0] !== 'key_key.command:key.keyboard.slash')) fail('Baseline Command binding is not slash');
  return Buffer.from(text.replace(/(^|[\r\n])key_key\.back:key\.keyboard\.s(?=$|[\r\n])/, '$1key_key.back:key.keyboard.k'));
}
function expectedKeydSnapshot() {
  return [
    ['mcsr-toolkit', sha('[mcsr_chat]\ns = s\nslash = slash\nenter = clearm(enter)\nesc = clearm(esc)\n')],
    ['mcsr-toolkit.conf', sha('# MCSR Toolkit managed\n[ids]\n*\n\ninclude mcsr-toolkit\n')]
  ];
}
function servePort(argv) {
  if (argv[0] !== '/usr/local/bin/mcsr-toolkit' || argv[1] !== 'serve') fail('Service command mismatch');
  let port = 8765, seenPort = false, seenLive = false;
  for (let i = 2; i < argv.length; i++) {
    if (argv[i] === '--enable-live-keyd' && !seenLive) seenLive = true;
    else if (argv[i] === '--port' && !seenPort && ++i < argv.length && /^[0-9]+$/.test(argv[i]) &&
             Number(argv[i]) >= 1 && Number(argv[i]) <= 65535) { port = Number(argv[i]); seenPort = true; }
    else fail('Unexpected service argument');
  }
  return port;
}
function listeningSocketInodes(tcp, port) {
  const address = `0100007F:${port.toString(16).toUpperCase().padStart(4, '0')}`;
  return new Set(tcp.trim().split('\n').slice(1).map(line => line.trim().split(/\s+/))
    .filter(fields => fields[1] === address && fields[3] === '0A' && /^[0-9]+$/.test(fields[9] ?? ''))
    .map(fields => fields[9]));
}
async function checkListeningSocket(pid, port) {
  const inodes = listeningSocketInodes(await fs.readFile(`/proc/${pid}/net/tcp`, 'utf8'), port);
  if (!inodes.size) fail('Service has no listening socket on the requested URL port');
  for (const fd of await fs.readdir(`/proc/${pid}/fd`)) {
    let target;
    try { target = await fs.readlink(`/proc/${pid}/fd/${fd}`); }
    catch (error) { if (error.code === 'ENOENT') continue; throw error; }
    if (inodes.has(/^socket:\[([0-9]+)\]$/.exec(target)?.[1])) return;
  }
  fail('Service MainPID does not own the requested URL listener');
}
async function regularFile(p) {
  const stat = await fs.lstat(p);
  if (!stat.isFile() || stat.isSymbolicLink()) fail(`Unsafe file: ${p}`);
  return stat;
}
async function keydSnapshot() {
  const dir = '/etc/keyd';
  const walk = async (where, prefix = '') => {
    const result = [];
    for (const entry of (await fs.readdir(where, { withFileTypes: true })).sort((a,b) => a.name.localeCompare(b.name))) {
      const name = `${prefix}${entry.name}`;
      const full = path.join(where, entry.name);
      if (entry.isDirectory()) result.push([name, 'dir'], ...await walk(full, `${name}/`));
      else if (entry.isFile()) result.push([name, await hashFile(full)]);
      else fail(`Unsafe /etc/keyd entry: ${name}`);
    }
    return result;
  };
  return walk(dir);
}
function checkSystem() {
  if (run('systemctl', ['is-active', 'keyd']) !== 'active') fail('keyd is not active');
  const unitFileState = run('systemctl', ['show', '-p', 'UnitFileState', '--value', 'keyd']);
  if (!unitFileState || unitFileState.startsWith('masked')) fail('keyd is masked or unavailable');
}
async function checkGameClosed() {
  for (const pid of await fs.readdir('/proc')) {
    if (!/^\d+$/.test(pid) || Number(pid) === process.pid) continue;
    let command;
    try { command = (await fs.readFile(`/proc/${pid}/cmdline`)).toString().replaceAll('\0', ' '); }
    catch (error) { if (error.code === 'ENOENT' || error.code === 'EACCES') continue; throw error; }
    if (/net\.minecraft\.client\.main\.Main|net\.fabricmc\.loader\.impl\.launch\.knot\.KnotClient|net\.fabricmc\.loader\.launch\.knot\.KnotClient|net\.minecraft\.launchwrapper\.Launch/i.test(command)) fail(`Possible Minecraft process remains: PID ${pid}`);
  }
}
function checkStatus(status, expectedLive) {
  if (status.live_activation_enabled !== expectedLive) fail(`Service live mode must be ${expectedLive}`);
  const focus = status.focus ?? {};
  // Runtime reports observe-only until a live profile has actually been applied.
  if (focus.mode !== 'observe-only') fail('Focus service must report observe-only before Apply or Restore');
  if (focus.extension_connected !== true || focus.minecraft_focused !== false ||
      focus.mutation_recovery_pending || focus.recovery_clear_pending || focus.unsafe_stop_failure ||
      focus.activation_suspended || focus.cleanup_error || focus.maccel?.recovery_pending ||
      (focus.errors?.length ?? 0) || (focus.fifo_errors?.length ?? 0)) fail('Bridge/recovery/error preflight failed');
  if (status.launcher_root !== root) fail('Launcher root mismatch');
  if (status.instances?.length !== 1 || status.instances[0].id !== instanceId ||
      status.instances[0].problem || status.instances[0].options_path !== optionsPath) fail('Expected sole healthy instance mismatch');
}
async function checkStateDirectory() {
  const parent = path.dirname(statePath);
  const stat = await fs.lstat(parent);
  if (!stat.isDirectory() || stat.isSymbolicLink() || stat.uid !== process.getuid() ||
      (stat.mode & 0o077) !== 0 || await fs.realpath(parent) !== parent) {
    fail('State directory must be a real, user-owned private directory (0700 or stricter)');
  }
}
async function syncStateDirectory() {
  const handle = await fs.open(path.dirname(statePath), 'r');
  try { await handle.sync(); } finally { await handle.close(); }
}
async function cleanLineage(root, profilePath) {
  const stateDir = path.dirname(profilePath);
  if (path.basename(profilePath) !== 'profile.json' || path.basename(stateDir) !== 'rust-v1' ||
      path.basename(path.dirname(stateDir)) !== 'mcsr-toolkit' || await fs.realpath(stateDir) !== stateDir) {
    fail('Profile path must be the canonical Rust state profile.json');
  }
  const record = async name => {
    const file = path.join(stateDir, name);
    try { await regularFile(file); return JSON.parse(await fs.readFile(file, 'utf8')); }
    catch (error) { if (error.code === 'ENOENT') return undefined; throw error; }
  };
  const lineage = await record('lineage.json');
  const managed = await record('managed.json');
  if (lineage !== undefined && !same(lineage, { head:null, profile:null, keyd:[], launcher_root:root })) fail('Rust lineage is not at unmanaged baseline');
  if (managed !== undefined && !same(managed, {})) fail('Rust managed state is not empty');
  for (const name of ['mutation.lock', 'mutation.json', 'keyd.json', 'keyd-uncertain.json', 'active-profile.json', 'pending.json']) {
    const file = path.join(stateDir, name);
    try { await fs.lstat(file); fail(`Rust ${name} exists at baseline`); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
  }
}
async function writeState(data, exclusive = false) {
  const body = JSON.stringify(data, null, 2) + '\n';
  if (exclusive) {
    const handle = await fs.open(statePath, 'wx', 0o600);
    try { await handle.writeFile(body); await handle.sync(); } finally { await handle.close(); }
    await syncStateDirectory();
    return;
  }
  await regularFile(statePath);
  const temp = `${statePath}.${randomUUID()}.tmp`;
  const handle = await fs.open(temp, 'wx', 0o600);
  try { await handle.writeFile(body); await handle.sync(); } finally { await handle.close(); }
  await fs.rename(temp, statePath);
  await syncStateDirectory();
}
async function readState() {
  const stat = await regularFile(statePath);
  if ((stat.mode & 0o077) !== 0) fail('State file must be private mode 0600');
  return JSON.parse(await fs.readFile(statePath, 'utf8'));
}
class CDP {
  constructor(url) {
    this.ws = new WebSocket(url);
    this.pending = new Map(); this.requests = new Map(); this.responses = new Map(); this.finished = new Map(); this.next = 1;
    this.ws.addEventListener('message', event => this.message(JSON.parse(event.data)));
  }
  async ready() {
    if (this.ws.readyState === WebSocket.OPEN) return;
    await new Promise((resolve, reject) => { this.ws.addEventListener('open', resolve, { once: true }); this.ws.addEventListener('error', reject, { once: true }); });
  }
  message(msg) {
    if (msg.id) { const p = this.pending.get(msg.id); if (p) { this.pending.delete(msg.id); msg.error ? p.reject(new Error(msg.error.message)) : p.resolve(msg.result); } return; }
    if (msg.method === 'Network.requestWillBeSent') this.requests.set(msg.params.requestId, msg.params.request.method);
    if (msg.method === 'Network.responseReceived') {
      const response = msg.params.response;
      this.responses.set(msg.params.requestId, { url: response.url, status: response.status, method: this.requests.get(msg.params.requestId) });
    }
    if (msg.method === 'Network.loadingFinished') this.finished.set(msg.params.requestId, Date.now());
  }
  send(method, params = {}) {
    const id = this.next++;
    return new Promise((resolve, reject) => { this.pending.set(id, { resolve, reject }); this.ws.send(JSON.stringify({ id, method, params })); });
  }
  async evaluate(expression) {
    const result = await this.send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) fail(result.exceptionDetails.text);
    return result.result.value;
  }
  async waitForResponse(endpoint, method, since, timeout = timeoutMs) {
    const deadline = Date.now() + timeout;
    for (;;) {
      for (const [id, finishedAt] of this.finished) {
        const entry = this.responses.get(id);
        if (finishedAt < since || !entry || entry.method !== method ||
            new URL(entry.url).origin !== baseUrl.origin || new URL(entry.url).pathname !== endpoint) continue;
        this.finished.delete(id);
        const raw = await this.send('Network.getResponseBody', { requestId: id });
        const text = raw.base64Encoded ? Buffer.from(raw.body, 'base64').toString() : raw.body;
        return { status: entry.status, body: JSON.parse(text) };
      }
      if (Date.now() > deadline) fail(`Timed out waiting for ${endpoint}; retain state and inspect service before logout`);
      await sleep(100);
    }
  }
  async click(id) {
    const result = await this.evaluate(`(() => { const x=document.getElementById(${JSON.stringify(id)}); if (!x || x.disabled || !x.getClientRects().length) return false; x.click(); return true; })()`);
    if (!result) fail(`UI button ${id} unavailable`);
  }
  close() { this.ws.close(); }
}
function childExited(child) { return child.exitCode !== null || child.signalCode !== null || child.spawnError !== undefined; }
function waitForChildExit(child, timeoutMs) {
  if (childExited(child)) return Promise.resolve(true);
  return new Promise(resolve => {
    const finish = exited => {
      clearTimeout(timer);
      child.removeListener('exit', onExit);
      child.removeListener('error', onError);
      resolve(exited);
    };
    const onExit = () => finish(true);
    const onError = () => finish(childExited(child));
    const timer = setTimeout(() => finish(childExited(child)), timeoutMs);
    child.once('exit', onExit);
    child.once('error', onError);
  });
}
async function stopChrome(chrome) {
  if (childExited(chrome)) return true;
  chrome.kill();
  if (await waitForChildExit(chrome, 5000)) return true;
  chrome.kill('SIGKILL');
  return waitForChildExit(chrome, 5000);
}
async function removeTemporaryProfile(dataDir) {
  for (let attempt = 0; ; attempt++) {
    try { await fs.rm(dataDir, { recursive:true, force:true }); return; }
    catch (error) {
      if (!['ENOTEMPTY', 'EBUSY', 'ENOTDIR'].includes(error.code) || attempt === 4) throw error;
      await sleep(100 * (attempt + 1));
    }
  }
}
async function closeTemporaryBrowser(cdp, chrome, dataDir) {
  try { cdp?.close(); }
  catch (error) { console.error(`WARNING: Could not close Chrome DevTools socket: ${error.message}`); }
  if (!(await stopChrome(chrome))) {
    console.error(`WARNING: Chrome did not exit; temporary profile retained at ${dataDir}`);
    return;
  }
  try { await removeTemporaryProfile(dataDir); }
  catch (error) { console.error(`WARNING: Could not remove temporary Chrome profile ${dataDir}: ${error.message}`); }
}
function spawnChrome(executable, dataDir) {
  const chrome = spawn(executable, ['--headless=new', '--no-first-run', '--no-default-browser-check', '--remote-debugging-port=0', `--user-data-dir=${dataDir}`, 'about:blank'], { stdio:'ignore' });
  chrome.on('error', error => { if (chrome.pid === undefined) chrome.spawnError = error; });
  return chrome;
}
async function browser() {
  const dataDir = await fs.mkdtemp(path.join(os.tmpdir(), 'mcsr-live-ui-'));
  const chrome = spawnChrome(flags.get('--chrome') ?? '/usr/bin/google-chrome', dataDir);
  let cdp;
  try {
    let port;
    for (let i=0;i<100;i++) { if (chrome.spawnError) throw chrome.spawnError; if (chrome.exitCode !== null) fail('Chrome exited'); try { port = (await fs.readFile(path.join(dataDir, 'DevToolsActivePort'), 'utf8')).split('\n')[0]; break; } catch { await sleep(100); } }
    if (!port) fail('Chrome debugging port unavailable');
    const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    const target = targets.find(x => x.type === 'page');
    if (!target) fail('Chrome page target unavailable');
    cdp = new CDP(target.webSocketDebuggerUrl); await cdp.ready();
    await cdp.send('Page.enable'); await cdp.send('Network.enable'); await cdp.send('Runtime.enable');
    await cdp.send('Page.navigate', { url: baseUrl.href });
    const deadline = Date.now() + timeoutMs;
    while (!(await cdp.evaluate(`document.readyState === 'complete' && !!document.querySelector('#save-profile') && !!document.querySelector('#status strong')`))) {
      if (Date.now() > deadline) fail('UI did not load'); await sleep(100);
    }
    return { cdp, async close() { await closeTemporaryBrowser(cdp, chrome, dataDir); } };
  } catch (error) { await closeTemporaryBrowser(cdp, chrome, dataDir); throw error; }
}
async function uiApi(cdp, endpoint) {
  return cdp.evaluate(`fetch(${JSON.stringify(endpoint)}, {credentials:'same-origin'}).then(async r => {if (!r.ok) throw Error('HTTP '+r.status); return r.json()})`);
}
async function clickResponse(cdp, button, endpoint) {
  const since = Date.now(); await cdp.click(button);
  const response = await cdp.waitForResponse(endpoint, endpoint === '/api/profile' ? 'PUT' : 'POST', since);
  if (response.status < 200 || response.status >= 300) fail(`${endpoint} returned ${response.status}: ${JSON.stringify(response.body)}`);
  return response.body;
}
async function waitForUi(cdp, expression, label) {
  const deadline = Date.now() + timeoutMs;
  while (!(await cdp.evaluate(expression))) {
    if (Date.now() > deadline) fail(`UI did not show ${label}; retain state and inspect before logout`);
    await sleep(100);
  }
}
async function terminal(cdp, id) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const job = await uiApi(cdp, `/api/jobs/${encodeURIComponent(id)}`);
    if (job.status !== 'running') return job;
    if (Date.now() > deadline) fail(`Job ${id} still running; retain state and inspect service before logout`);
    await sleep(700);
  }
}
async function checkServiceIdentity() {
  const rawPid = run('systemctl', ['--user', 'show', '-P', 'MainPID', 'mcsr-toolkit.service']);
  if (!/^[1-9][0-9]*$/.test(rawPid)) fail('Toolkit user service has no MainPID');
  const pid = Number(rawPid);
  const exe = await fs.stat(`/proc/${pid}/exe`);
  const installed = await fs.stat('/usr/local/bin/mcsr-toolkit');
  if (exe.dev !== installed.dev || exe.ino !== installed.ino || exe.uid !== 0) fail('Service is not running the installed root-owned executable');
  const command = (await fs.readFile(`/proc/${pid}/cmdline`)).toString().split('\0').filter(Boolean);
  const servicePort = servePort(command);
  if (servicePort !== Number(urlPort)) fail('Requested URL port differs from the service command');
  await checkListeningSocket(pid, servicePort);
  const values = (await fs.readFile(`/proc/${pid}/environ`)).toString().split('\0').filter(Boolean);
  const env = new Map(values.map(item => [item.slice(0, item.indexOf('=')), item.slice(item.indexOf('=') + 1)]));
  const home = env.get('HOME');
  if (!home || !path.isAbsolute(home)) fail('Service HOME is unavailable');
  const dataHome = env.get('XDG_DATA_HOME') || path.join(home, '.local/share');
  if (!path.isAbsolute(dataHome)) fail('Service XDG_DATA_HOME is not absolute');
  const expectedProfile = path.join(await fs.realpath(dataHome), 'mcsr-toolkit/rust-v1/profile.json');
  if (profilePath !== expectedProfile) fail('Supplied profile path is not the running service state profile');
  if (run('systemctl', ['--user', 'show', '-P', 'MainPID', 'mcsr-toolkit.service']) !== rawPid) fail('Toolkit service restarted during identity check');
  return pid;
}
async function preflight(cdp, live) {
  const pid = await checkServiceIdentity();
  checkSystem(); await checkGameClosed();
  if (await canonical(root) !== root || await canonical(optionsPath) !== optionsPath || await canonical(profilePath) !== profilePath) fail('Paths must be canonical real paths');
  if (!optionsPath.startsWith(`${root}/`)) fail('Options path is outside launcher root');
  checkStatus(await uiApi(cdp, '/api/status'), live);
  return pid;
}
async function cleanupProfile(state, cdp) {
  const target = flags.get('--cleanup-profile');
  if (!target) return;
  const current = await uiApi(cdp, '/api/profile');
  if (target === 'absent') {
    try { await fs.lstat(profilePath); } catch (error) { if (error.code === 'ENOENT') return; throw error; }
  }
  if (target !== 'absent') {
    const replacement = JSON.parse(await fs.readFile(target, 'utf8'));
    if (same(replacement, current)) return;
  }
  if (!same(current, state.testProfile)) fail('Profile changed; cleanup refused');
  if (target === 'absent') {
    const currentFile = JSON.parse(await fs.readFile(profilePath, 'utf8'));
    if (!same(currentFile, state.testProfile)) fail('Profile file changed; cleanup refused');
    await fs.unlink(profilePath);
  } else {
    const replacement = JSON.parse(await fs.readFile(target, 'utf8'));
    // Use the application's validated write path and compare immediately before it.
    const put = await cdp.evaluate(`(async()=>{const before=await (await fetch('/api/profile')).json(); const norm=x=>Array.isArray(x)?x.map(norm):x&&typeof x==='object'?Object.fromEntries(Object.keys(x).sort().map(k=>[k,norm(x[k])])):x; if (JSON.stringify(norm(before))!==${JSON.stringify(JSON.stringify(stable(state.testProfile)))}) throw Error('Profile changed'); const token=(await (await fetch('/api/session')).json()).token; const r=await fetch('/api/profile',{method:'PUT',headers:{Origin:location.origin,'X-MCSR-Session':token,'Content-Type':'application/json'},body:${JSON.stringify(JSON.stringify(replacement))}}); if(!r.ok) throw Error('Profile cleanup PUT '+r.status); return r.json()})()`);
    if (!same(put, replacement)) fail('Profile cleanup response mismatch');
  }
}
async function main() {
  await checkStateDirectory();
  const stored = phase === 'restore' ? await readState() : null;
  const baselineOptions = phase === 'prepare' ? await fs.readFile(optionsPath) : null;
  if (phase === 'prepare') {
    try { await fs.lstat(statePath); fail('State file already exists'); } catch (error) { if (error.code !== 'ENOENT') throw error; }
    if (sha(baselineOptions) !== baselineSha) fail('Options baseline SHA mismatch');
    expectedAppliedOptions(baselineOptions);
    if ((await keydSnapshot()).length) fail('/etc/keyd is not empty');
    await cleanLineage(root, profilePath);
  } else {
    if (stored.version !== 1 || !['applied', 'restored'].includes(stored.phase) || stored.root !== root || stored.instanceId !== instanceId || stored.optionsPath !== optionsPath || stored.profilePath !== profilePath || stored.baselineSha !== baselineSha || stored.url !== baseUrl.href) fail('Private state/arguments mismatch');
    if (stored.phase === 'applied' && (await hashFile(optionsPath) !== stored.postSha || !same(await keydSnapshot(), stored.postKeyd))) fail('Post-Apply options/keyd snapshot mismatch');
    if (stored.phase === 'restored' && (await hashFile(optionsPath) !== baselineSha || (await keydSnapshot()).length)) fail('Prior Restore result has drifted');
  }
  if (stored?.phase === 'restored') {
    await cleanLineage(root, profilePath);
    console.log(`Already restored; baseline SHA and empty /etc/keyd verified. Private state retained: ${statePath}`);
    return;
  }
  const b = await browser();
  try {
    const cdp = b.cdp;
    const servicePid = await preflight(cdp, phase === 'prepare');
    if (phase === 'prepare') {
      const profile = JSON.parse(await fs.readFile(get('profile-json'), 'utf8'));
      const loaded = await uiApi(cdp, '/api/profile');
      if (!same(loaded, profile) || !same(profile, expectedProfile(root))) fail('Pre-saved exact S→K profile mismatch');
      const saved = await clickResponse(cdp, 'save-profile', '/api/profile');
      if (!same(saved, profile)) fail('UI Save profile mismatch');
      const preview = await clickResponse(cdp, 'preview-profile', '/api/preview');
      if (!validPreview(preview, instanceId)) fail('UI Preview is not the exact ready S→K mapping');
      if (await preflight(cdp, true) !== servicePid) fail('Service restarted during Prepare');
      if (await hashFile(optionsPath) !== baselineSha || (await keydSnapshot()).length) fail('Baseline changed before Apply');
      await cleanLineage(root, profilePath);
      const state = { version:1, phase:'apply-submitting', url:baseUrl.href, root, instanceId, optionsPath, profilePath, baselineSha, testProfile:profile, createdAt:new Date().toISOString() };
      await writeState(state, true);
      if (await checkServiceIdentity() !== servicePid) fail('Service changed before Apply');
      const accepted = await clickResponse(cdp, 'apply-profile', '/api/apply');
      if (!accepted.id) fail('Apply acceptance lacked a job ID; inspect service before logout');
      state.applyJobId = accepted.id; state.phase = 'apply-running'; await writeState(state);
      const job = await terminal(cdp, accepted.id);
      state.applyJob = job;
      if (!validApplyJob(job, instanceId)) { await writeState(state); fail('Apply failed or ambiguous; preserve state and do not logout'); }
      state.backupId = job.result.backup_id; await writeState(state);
      await waitForUi(cdp, `document.getElementById('backup-id').value === ${JSON.stringify(job.result.backup_id)}`, 'Apply backup ID');
      const appliedOptions = await fs.readFile(optionsPath);
      state.postSha = sha(appliedOptions);
      state.postKeyd = await keydSnapshot();
      if (!appliedOptions.equals(expectedAppliedOptions(baselineOptions)) || !same(state.postKeyd, expectedKeydSnapshot())) {
        await writeState(state); fail('Apply did not produce the exact S→K options/keyd changes');
      }
      if (await checkServiceIdentity() !== servicePid) fail('Service restarted during Apply');
      state.phase = 'applied'; await writeState(state);
      console.log(`Apply succeeded. Backup ${state.backupId}; job ${accepted.id}. Private state: ${statePath}. Restore requires observe-only service.`);
    } else {
      if (!stored.backupId || !same(await uiApi(cdp, '/api/profile'), stored.testProfile)) fail('Restore state/profile mismatch');
      await cdp.evaluate(`document.getElementById('backup-id').value=${JSON.stringify(stored.backupId)}`);
      stored.phase = 'restore-submitting'; await writeState(stored);
      if (await checkServiceIdentity() !== servicePid) fail('Service changed before Restore');
      const accepted = await clickResponse(cdp, 'restore-backup', '/api/restore');
      if (!accepted.id) fail('Restore acceptance lacked a job ID; preserve state');
      stored.restoreJobId = accepted.id; stored.phase = 'restore-running'; await writeState(stored);
      const job = await terminal(cdp, accepted.id); stored.restoreJob = job; await writeState(stored);
      if (job.status !== 'succeeded' || job.result?.backup_id !== stored.backupId || job.result.instances?.length !== 1 || job.result.instances[0].status !== 'restored') fail('Restore failed or ambiguous; preserve state');
      await waitForUi(cdp, `document.getElementById('restore-results').textContent.includes(${JSON.stringify(instanceId)})`, 'Restore result');
      if (await hashFile(optionsPath) !== baselineSha || (await keydSnapshot()).length) fail('Restore baseline/keyd verification failed; preserve state');
      await cleanLineage(root, profilePath);
      if (await preflight(cdp, false) !== servicePid) fail('Service restarted during Restore');
      await cleanupProfile(stored, cdp);
      stored.phase = 'restored'; stored.restoredAt = new Date().toISOString(); await writeState(stored);
      console.log(`UI Restore verified baseline SHA and empty /etc/keyd. Private state retained: ${statePath}`);
    }
  } finally { await b.close(); }
}
main().catch(error => { console.error(`STOP: ${error.message}`); process.exitCode = 1; });
