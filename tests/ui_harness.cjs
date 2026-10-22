const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

// Exercise the actual event handlers with a small DOM and controlled network replies.
class Node {
  constructor(tag = 'div') {
    this.tag = tag;
    this.children = [];
    this.listeners = new Map();
    this.dataset = {};
    this.value = '';
    this.text = '';
  }
  set textContent(value) { this.text = String(value); }
  get textContent() { return this.text + this.children.map(node => node.textContent).join(''); }
  append(...nodes) {
    for (const node of nodes) { node.parent = this; this.children.push(node); }
  }
  replaceChildren(...nodes) { this.children = []; this.text = ''; this.append(...nodes); }
  setAttribute() {}
  addEventListener(name, callback) { this.listeners.set(name, callback); }
  emit(name) { return this.listeners.get(name)?.({preventDefault() {}, target: this}); }
  remove() { this.parent.children = this.parent.children.filter(node => node !== this); }
  querySelectorAll(selector) {
    const matches = node => selector === 'input' ? node.tag === 'input'
      : node.dataset.field === selector.match(/^\[data-field="(.*)"\]$/)?.[1];
    return this.children.flatMap(node => [ ...(matches(node) ? [node] : []),
      ...node.querySelectorAll(selector) ]);
  }
  querySelector(selector) { return this.querySelectorAll(selector)[0] ?? null; }
}

const source = fs.readFileSync('static/app.js', 'utf8');
const profile = target => ({controls: {}, crafting: {s: target}, ninjabrain: {}, snippets: [],
  chat: {open_chat: 't', open_command: 'slash', send: 'enter', cancel: 'esc'},
  presets: [], launcher_root: '/launcher'});
const tick = () => new Promise(resolve => setImmediate(resolve));

async function browser({deferProfile = false} = {}) {
  const ids = new Map();
  const chats = new Map();
  const get = id => {
    if (!ids.has(id)) ids.set(id, new Node());
    return ids.get(id);
  };
  const document = {
    getElementById: get,
    createElement: tag => new Node(tag),
    createTextNode: text => { const node = new Node('text'); node.textContent = text; return node; },
    querySelectorAll: () => [],
    querySelector: selector => {
      const name = selector.match(/^\[name="(.*)"\]$/)[1];
      if (!chats.has(name)) chats.set(name, new Node('input'));
      return chats.get(name);
    }
  };
  const calls = [];
  let canonical = profile('j');
  let jobPath = '';
  let profileGets = 0;
  let resolveProfile;
  const fetch = async (path, options = {}) => {
    const data = options.body ? JSON.parse(options.body) : undefined;
    calls.push({path, method: options.method, data});
    let body;
    if (path === '/api/session') body = {token: 'session'};
    else if (path === '/api/status') body = {live_activation_enabled: true,
      launcher_root: '/launcher', instances: [{id: 'one'}], focus: {mode: 'live'}};
    else if (path === '/api/profile' && options.method === 'PUT') {
      canonical = data; body = canonical;
    } else if (path === '/api/profile') {
      profileGets++;
      if (deferProfile && profileGets > 1) await new Promise(resolve => {resolveProfile = resolve;});
      body = canonical;
    } else if (path === '/api/preview') body = {instances: [{instance_id: 'one', status: 'ready'}]};
    else if (path === '/api/restore' || path === '/api/apply') {
      jobPath = path;
      if (path === '/api/restore') canonical = profile('k');
      body = {id: 'job'};
    } else if (path.startsWith('/api/jobs/')) body = {status: 'succeeded', result: {
      instances: [{instance_id: 'one', status: jobPath === '/api/restore' ? 'restored' : 'updated'}]
    }};
    else throw new Error(`unexpected API request ${path}`);
    return {ok: true, json: async () => JSON.parse(JSON.stringify(body))};
  };
  const context = {document, window: {location: {origin: 'http://127.0.0.1:8765'}}, fetch,
    setTimeout: callback => {callback(); return 0;}};
  vm.runInNewContext(source, context);
  await tick();
  get('backup-id').value = 'backup-b';
  const craftingTarget = () => get('crafting-rows').children[0].querySelectorAll('input')[1];
  return {get, calls, craftingTarget, resolveProfile: () => resolveProfile(),
    edit: value => {craftingTarget().value = value; get('profile-form').emit('input');}};
}

(async () => {
  const clean = await browser();
  await clean.get('restore-backup').emit('click');
  assert.equal(clean.craftingTarget().value, 'k', 'restore must show the canonical prior profile');
  await clean.get('preview-profile').emit('click');
  assert.equal(clean.get('apply-profile').disabled, false);
  await clean.get('apply-profile').emit('click');
  assert.equal(clean.calls.find(call => call.path === '/api/apply').data.crafting.s, 'k',
    'Apply must use the restored saved profile');

  const dirty = await browser();
  dirty.edit('h');
  await dirty.get('restore-backup').emit('click');
  assert.equal(dirty.craftingTarget().value, 'h', 'restore must preserve an existing unsaved draft');
  await dirty.get('preview-profile').emit('click');
  assert.equal(dirty.get('apply-profile').disabled, true, 'draft must be saved after canonical changes');

  const racing = await browser({deferProfile: true});
  const restore = racing.get('restore-backup').emit('click');
  await tick();
  racing.edit('h');
  racing.resolveProfile();
  await restore;
  assert.equal(racing.craftingTarget().value, 'h', 'in-flight profile refresh must preserve new edits');
  await racing.get('preview-profile').emit('click');
  assert.equal(racing.get('apply-profile').disabled, true);
  console.log('UI restore profile and draft race checks passed');
})().catch(error => {console.error(error); process.exitCode = 1;});
