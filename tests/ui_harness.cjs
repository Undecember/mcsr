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
  get parentElement() { return this.parent; }
  get isConnected() { return !this.removed && (!this.parent || this.parent.isConnected); }
  setAttribute() {}
  addEventListener(name, callback) { this.listeners.set(name, callback); }
  emit(name, properties = {}) { return this.listeners.get(name)?.({preventDefault() {}, stopPropagation() {}, stopImmediatePropagation() {}, target: this, ...properties}); }
  remove() { this.removed = true; this.parent.children = this.parent.children.filter(node => node !== this); }
  querySelectorAll(selector) {
    const matches = node => ['input', 'select', 'option', 'button'].includes(selector) ? node.tag === selector
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

async function browser({deferProfile = false, initialProfile, initialFocus = {}} = {}) {
  const ids = new Map();
  const chats = new Map();
  const get = id => {
    if (!ids.has(id)) ids.set(id, new Node());
    return ids.get(id);
  };
  const document = Object.assign(new Node('document'), {
    getElementById: get,
    createElement: tag => new Node(tag),
    createTextNode: text => { const node = new Node('text'); node.textContent = text; return node; },
    querySelectorAll: () => [],
    querySelector: selector => {
      const name = selector.match(/^\[name="(.*)"\]$/)[1];
      if (!chats.has(name)) {
        const input = new Node('input');
        new Node('div').append(input);
        chats.set(name, input);
      }
      return chats.get(name);
    }
  });
  const calls = [];
  let canonical = initialProfile || profile('j');
  let jobPath = '';
  let profileGets = 0;
  let resolveProfile;
  const fetch = async (path, options = {}) => {
    const data = options.body ? JSON.parse(options.body) : undefined;
    calls.push({path, method: options.method, data});
    let body;
    if (path === '/api/session') body = {token: 'session'};
    else if (path === '/api/status') body = {live_activation_enabled: true,
      launcher_root: '/launcher', instances: [{id: 'one'}], focus: {mode: 'live', ...initialFocus}};
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
  const window = Object.assign(new Node('window'), {location: {origin: 'http://127.0.0.1:8765'}});
  const intervals = new Set();
  const context = {document, window, fetch,
    setInterval: callback => {intervals.add(callback); return callback;},
    clearInterval: callback => intervals.delete(callback),
    setTimeout: callback => {callback(); return 0;}};
  vm.runInNewContext(source, context);
  await tick();
  get('backup-id').value = 'backup-b';
  const craftingTarget = () => get('crafting-rows').children[0].querySelectorAll('input')[1];
  return {get, calls, document, chats, craftingTarget, resolveProfile: () => resolveProfile(),
    edit: value => {craftingTarget().value = value; get('profile-form').emit('input');}};
}

(async () => {
  const fullSize = await browser({initialFocus: {window_mode: 'full_size', active_preset: null}});
  assert.match(fullSize.get('status').textContent, /Full-size borderless/,
    'a verified full-size result must be shown as a borderless window');
  const presetSize = await browser({initialFocus: {window_mode: 'preset', active_preset: 'thin'}});
  assert.match(presetSize.get('status').textContent, /Preset: thin/,
    'a preset-sized window must identify its active preset');

  const clean = await browser();
  assert.doesNotMatch(clean.get('status').textContent, /Full-size borderless|Preset: /,
    'missing resize state must not invent a window mode');
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
  // Catch dropping IDs during picker serialization, or failing to invalidate a preview.
  const controls = await browser({initialProfile: {...profile('j'), controls: {
    'key_key.hotbar.1': 'z', 'key_mod.toggle': 'v'
  }}});
  const controlRows = () => controls.get('controls-rows').children;
  const picker = controlRows()[0].querySelector('select');
  assert.ok(picker, 'Minecraft controls must use a picker');
  assert.equal(picker.value, 'key_key.hotbar.1');
  assert.match(picker.querySelectorAll('option').find(option => option.value === picker.value).textContent,
    /1 — Hotbar slot 1/);
  assert.equal(controlRows()[1].querySelector('select').value, '__custom__');
  assert.equal(controlRows()[1].querySelector('[data-field="custom-control"]').value, 'key_mod.toggle');
  await controls.get('save-profile').emit('click');
  const saves = () => controls.calls.filter(call => call.path === '/api/profile' && call.method === 'PUT');
  assert.deepEqual(saves().at(-1).data.controls, {'key_key.hotbar.1': 'z', 'key_mod.toggle': 'v'});
  await controls.get('preview-profile').emit('click');
  assert.equal(controls.get('apply-profile').disabled, false);
  const search = controlRows()[0].querySelector('[data-field="control-search"]');
  search.value = 'jump';
  search.emit('input');
  assert.equal(picker.value, 'key_key.hotbar.1', 'filtering must preserve the selected control');
  assert.equal(controls.get('apply-profile').disabled, false, 'searching alone must preserve the preview');
  assert.ok(picker.querySelectorAll('option').some(option => option.value === 'key_key.jump'));
  assert.equal(picker.querySelectorAll('option').some(option => option.value === 'key_key.back'), false);
  picker.value = 'key_key.jump';
  picker.emit('change');
  assert.equal(controls.get('apply-profile').disabled, true, 'selecting another control invalidates preview');
  await controls.get('save-profile').emit('click');
  assert.deepEqual(saves().at(-1).data.controls, {'key_key.jump': 'z', 'key_mod.toggle': 'v'});

  picker.value = '__custom__';
  picker.emit('change');
  const custom = controlRows()[0].querySelector('[data-field="custom-control"]');
  assert.equal(custom.disabled, false);
  await controls.get('save-profile').emit('click');
  assert.match(controls.get('notice').textContent, /Complete both fields/);
  custom.value = 'key_mod.another';
  await controls.get('save-profile').emit('click');
  assert.deepEqual(saves().at(-1).data.controls, {'key_mod.another': 'z', 'key_mod.toggle': 'v'});
  const reloaded = await browser({initialProfile: saves().at(-1).data});
  assert.equal(reloaded.get('controls-rows').children[0].querySelector('[data-field="custom-control"]').value,
    'key_mod.another');
  custom.value = 'key_mod.toggle';
  const beforeDuplicate = saves().length;
  await controls.get('save-profile').emit('click');
  assert.equal(saves().length, beforeDuplicate);
  assert.match(controls.get('notice').textContent, /Duplicate source/);
  picker.value = '';
  picker.emit('change');
  await controls.get('save-profile').emit('click');
  assert.match(controls.get('notice').textContent, /Complete both fields/);
  // Missing a capture button on any keystroke field, or adding one to an action,
  // would make the intended flow inaccessible or record the wrong profile value.
  const recording = await browser({initialProfile: {...profile('j'),
    controls: {'key_key.jump': 'space'}, ninjabrain: {q: 'leftcontrol+n'},
    presets: [{id: 'small', width: 800, height: 600, shortcut: 'f6'}]}});
  const recordButtons = node => node.querySelectorAll('button').filter(button => button.dataset.field === 'record');
  assert.equal(recordButtons(recording.get('controls-rows')).length, 1);
  assert.equal(recordButtons(recording.get('crafting-rows')).length, 2);
  assert.equal(recordButtons(recording.get('ninjabrain-rows')).length, 2);
  assert.equal(recordButtons(recording.get('presets-rows')).length, 1);
  assert.equal(recordButtons(recording.get('controls-rows').children[0].children[0]).length, 0,
    'Minecraft actions are selected, never captured');
  for (const input of recording.chats.values()) assert.equal(recordButtons(input.parentElement).length, 1);
  recordButtons(recording.get('crafting-rows'))[0].emit('click');
  recording.document.emit('mousedown', {button: 3});
  assert.equal(recording.get('crafting-rows').querySelector('input').value, 's',
    'Mouse sources require explicit keyd device enrollment');
  recording.document.emit('keydown', {code: 'KeyS'});
  await recording.get('preview-profile').emit('click');
  assert.equal(recording.get('apply-profile').disabled, false);
  const controlInput = recording.get('controls-rows').children[0].children[1].querySelector('input');
  recordButtons(recording.get('controls-rows'))[0].emit('click');
  assert.equal(controlInput.value, 'space', 'activation must not record itself');
  assert.equal(recording.get('apply-profile').disabled, false, 'starting alone is not an edit');
  recording.document.emit('keydown', {code: 'Backspace'});
  assert.equal(controlInput.value, 'backspace');
  assert.equal(recording.get('apply-profile').disabled, true, 'recording invalidates the preview');
  recordButtons(recording.get('crafting-rows'))[1].emit('click');
  recording.document.emit('mousedown', {button: 3});
  assert.equal(recording.craftingTarget().value, 'mouse1');
  const shortcut = recording.get('ninjabrain-rows').querySelectorAll('input')[1];
  recordButtons(recording.get('ninjabrain-rows'))[1].emit('click');
  recording.document.emit('keydown', {code: 'ControlRight', ctrlKey: true});
  recording.document.emit('mousedown', {button: 4, ctrlKey: true});
  assert.equal(shortcut.value, 'rightcontrol+mouse2');
  const chat = recording.chats.get('cancel');
  recordButtons(chat.parentElement)[0].emit('click');
  recording.document.emit('keydown', {code: 'Escape'});
  assert.equal(chat.value, 'esc');
  const preset = recording.get('presets-rows').querySelector('[data-field="shortcut"]');
  recordButtons(recording.get('presets-rows'))[0].emit('click');
  recording.document.emit('mousedown', {button: 1});
  assert.equal(preset.value, 'f6', 'GNOME shortcuts must reject mouse input');
  recording.document.emit('keydown', {code: 'F8', altKey: true});
  assert.equal(preset.value, 'leftalt+f8');
  await recording.get('save-profile').emit('click');
  const captured = recording.calls.filter(call => call.method === 'PUT').at(-1).data;
  assert.deepEqual(captured.controls, {'key_key.jump': 'backspace'});
  assert.equal(captured.crafting.s, 'mouse1');
  assert.equal(captured.ninjabrain.q, 'rightcontrol+mouse2');
  assert.equal(captured.presets[0].shortcut, 'leftalt+f8');
  console.log('UI restore, draft race, control picker, input recording, and profile round-trip checks passed');
})().catch(error => {console.error(error); process.exitCode = 1;});
