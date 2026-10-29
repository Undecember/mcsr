"use strict";

// Minecraft Java 1.16 defaults. IDs are options.txt keys, not physical keys.
const MINECRAFT_CONTROLS = [
  ['forward', 'W', 'Move forward'],
  ['back', 'S', 'Move backward'],
  ['left', 'A', 'Strafe left'],
  ['right', 'D', 'Strafe right'],
  ['jump', 'Space', 'Jump'],
  ['sneak', 'Left Shift', 'Sneak'],
  ['sprint', 'Left Control', 'Sprint'],
  ['attack', 'Left Mouse', 'Attack / Destroy'],
  ['use', 'Right Mouse', 'Use item / Place block'],
  ['pickItem', 'Middle Mouse', 'Pick block'],
  ['inventory', 'E', 'Open / Close inventory'],
  ['drop', 'Q', 'Drop selected item'],
  ['swapOffhand', 'F', 'Swap item with offhand'],
  ...Array.from({length: 9}, (_, index) =>
    [`hotbar.${index + 1}`, String(index + 1), `Hotbar slot ${index + 1}`]),
  ['chat', 'T', 'Open chat'],
  ['command', '/', 'Open command'],
  ['playerlist', 'Tab', 'List players'],
  ['advancements', 'L', 'Advancements'],
  ['screenshot', 'F2', 'Take screenshot'],
  ['togglePerspective', 'F5', 'Toggle perspective'],
  ['fullscreen', 'F11', 'Toggle fullscreen'],
  ['saveToolbarActivator', 'C', 'Save hotbar'],
  ['loadToolbarActivator', 'X', 'Load hotbar'],
  ['smoothCamera', 'Unbound', 'Toggle cinematic camera'],
  ['spectatorOutlines', 'Unbound', 'Highlight players (spectators)']
].map(([name, key, action]) => ({id: `key_key.${name}`, key, action}));

function findMinecraftControls(query) {
  const normalize = value => value.toLowerCase().replace(/[^a-z0-9/]/g, '');
  const term = normalize(query.trim());
  if (!term) return MINECRAFT_CONTROLS;
  const exactKey = MINECRAFT_CONTROLS.filter(control => normalize(control.key) === term);
  if (exactKey.length) return exactKey;
  return MINECRAFT_CONTROLS.filter(control =>
    normalize(control.key).includes(term) || normalize(control.action).includes(term));
}

function profileFromRows(rows) {
  const result = {};
  for (const [rawSource, rawTarget] of rows) {
    const source = rawSource.trim();
    const target = rawTarget.trim();
    if (!source && !target) continue;
    if (!source || !target) throw new Error('Complete both fields in each binding row.');
    if (Object.hasOwn(result, source)) throw new Error(`Duplicate source: ${source}`);
    result[source] = target;
  }
  return result;
}

function canApply(status, preview, savedRevision, currentRevision, busy) {
  return Boolean(status?.live_activation_enabled && !status.focus?.mutation_recovery_pending &&
    !busy && preview?.instances?.length &&
    savedRevision === currentRevision &&
    preview.instances.every(item => item.status === 'ready' || item.status === 'unchanged'));
}

const KEY_CODES = {
  Backspace: 'backspace', Tab: 'tab', Enter: 'enter', Escape: 'esc', Space: 'space',
  Minus: 'minus', Equal: 'equal', BracketLeft: 'leftbrace', BracketRight: 'rightbrace',
  Backslash: 'backslash', Semicolon: 'semicolon', Quote: 'apostrophe', Backquote: 'grave',
  Comma: 'comma', Period: 'dot', Slash: 'slash', CapsLock: 'capslock', NumLock: 'numlock',
  ScrollLock: 'scrolllock', PrintScreen: 'sysrq', Pause: 'pause', Insert: 'insert',
  Delete: 'delete', Home: 'home', End: 'end', PageUp: 'pageup', PageDown: 'pagedown',
  ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right', ContextMenu: 'menu',
  ShiftLeft: 'leftshift', ShiftRight: 'rightshift', ControlLeft: 'leftcontrol',
  ControlRight: 'rightcontrol', AltLeft: 'leftalt', AltRight: 'rightalt',
  MetaLeft: 'leftmeta', MetaRight: 'rightmeta', NumpadAdd: 'kpplus',
  NumpadSubtract: 'kpminus', NumpadMultiply: 'kpasterisk', NumpadDivide: 'kpslash',
  NumpadDecimal: 'kpdot', NumpadEnter: 'kpenter', NumpadEqual: 'kpequal'
};
const MOUSE_KEYS = ['leftmouse', 'middlemouse', 'rightmouse', 'mouse1', 'mouse2'];
const MODIFIERS = [['control', 'ctrlKey'], ['shift', 'shiftKey'], ['alt', 'altKey'], ['meta', 'metaKey']];

function keyboardKey(code) {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3).toLowerCase();
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^Numpad[0-9]$/.test(code)) return `kp${code.slice(6)}`;
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code.toLowerCase();
  return Object.hasOwn(KEY_CODES, code) ? KEY_CODES[code] : null;
}

// One recorder owns page input while listening. Keep the trailing mouse gesture
// suppressed after committing so it cannot click Remove, navigate, or open a menu.
function createInputRecorder(root, view, clock = {
  now: () => Date.now(), setInterval: (fn, ms) => setInterval(fn, ms),
  clearInterval: id => clearInterval(id)
}) {
  let active = null, trailing = null;
  const consume = event => { event.preventDefault(); event.stopImmediatePropagation(); };
  function stop(text = 'Recording stopped. Binding unchanged.') {
    if (!active) return;
    const previous = active;
    active = null;
    clock.clearInterval(previous.timer);
    previous.onStatus(text);
    previous.onStop?.();
  }
  function ready() {
    if (!active) return false;
    if (!active.input.isConnected) stop();
    else if (clock.now() >= active.deadline) stop('Recording timed out. Binding unchanged.');
    return Boolean(active);
  }
  function status(hint) {
    if (!ready()) return;
    if (hint !== undefined) active.hint = hint;
    const seconds = Math.ceil((active.deadline - clock.now()) / 1000);
    active.onStatus(`${active.hint || (active.allowMouse ? 'Press a key or mouse button…' : 'Press a key…')} ${seconds}s`);
  }
  function commit(key, event) {
    const current = active;
    let value = key;
    if (current.shortcut) {
      const modifiers = MODIFIERS.flatMap(([family, flag]) => {
        const held = [...current.modifiers].filter(name => name.endsWith(family));
        return event[flag] || held.length ? [held.at(-1) || `left${family}`] : [];
      });
      value = [...modifiers, key].join('+');
    }
    const different = current.input.value !== value;
    current.input.value = value;
    stop(`Recorded ${value}.`);
    if (different) current.onChange();
  }
  root.addEventListener('keydown', event => {
    if (!ready()) {
      if (trailing?.code === event.code) consume(event);
      return;
    }
    consume(event);
    if (event.repeat || event.isComposing) return;
    const key = keyboardKey(event.code);
    if (!key) { status('Unsupported key. Try another input or enter it manually.'); return; }
    const modifier = MODIFIERS.some(([family]) => key.endsWith(family));
    if (active.shortcut && modifier) {
      active.modifiers.add(key);
      status('Hold modifiers and press the shortcut key…');
      return;
    }
    trailing = {code: event.code, until: Infinity};
    commit(key, event);
  }, true);
  root.addEventListener('keyup', event => {
    if (ready()) {
      consume(event);
      active.modifiers.delete(keyboardKey(event.code));
    } else if (trailing?.code === event.code && clock.now() < trailing.until) {
      consume(event);
      trailing = null;
    }
  }, true);
  root.addEventListener('pointerdown', event => {
    trailing = null;
    // Canceling pointerdown suppresses compatibility mousedown in browsers.
    if (ready()) event.stopImmediatePropagation();
  }, true);
  root.addEventListener('mousedown', event => {
    trailing = null;
    if (!ready()) return;
    consume(event);
    trailing = {button: event.button, until: Infinity};
    const key = MOUSE_KEYS[event.button];
    if (!active.allowMouse || !key) {
      status(active.allowMouse ? 'Unsupported mouse button. Try another input.' :
        active.mouseReason || 'This shortcut requires a keyboard key.');
      return;
    }
    commit(key, event);
  }, true);
  for (const type of ['pointerup', 'mouseup', 'click', 'auxclick', 'contextmenu']) {
    root.addEventListener(type, event => {
      if (ready() || (trailing && clock.now() < trailing.until &&
          (event.button === trailing.button || (trailing.code && type === 'click')))) {
        consume(event);
        if (trailing && (type === 'mouseup' || type === 'pointerup')) trailing.until = clock.now() + 1000;
        if (!active && (type === 'click' || type === 'auxclick')) trailing = null;
      }
    }, true);
  }
  view.addEventListener('blur', () => {stop(); trailing = null;});
  return {
    start(options) {
      stop();
      trailing = null;
      active = {...options, modifiers: new Set(), deadline: clock.now() + 5000, hint: ''};
      status();
      active.timer = clock.setInterval(() => status(), 100);
    },
    stop
  };
}

if (typeof module !== 'undefined') module.exports = { canApply, profileFromRows, findMinecraftControls, keyboardKey, createInputRecorder };

if (typeof document !== 'undefined') {
  const $ = id => document.getElementById(id);
  const state = {
    token: '', status: null, preview: null, savedProfile: null,
    revision: 0, savedRevision: -1, busy: false
  };
  const fields = {
    controls: ['Minecraft control', 'Physical key'],
    crafting: ['Physical key', 'Emitted key'],
    ninjabrain: ['Physical key', 'Ninjabrain shortcut']
  };

  function element(tag, className, content) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (content !== undefined) node.textContent = String(content);
    return node;
  }

  function message(target, value, kind = '') {
    const box = $(target);
    box.replaceChildren();
    if (value) box.append(element('p', `message ${kind}`, value));
  }

  function changed() {
    state.revision++;
    state.preview = null;
    $('preview-results').replaceChildren();
    updateApply();
  }

  function updateApply() {
    $('apply-profile').disabled = !canApply(
      state.status, state.preview, state.savedRevision, state.revision, state.busy
    );
  }

  const recorder = createInputRecorder(document, window);

  function rowInput(label, value = '') {
    const wrapper = element('label');
    wrapper.append(element('span', 'sr-only', label));
    const input = element('input');
    input.type = 'text';
    input.value = value;
    input.placeholder = label;
    input.setAttribute('aria-label', label);
    wrapper.append(input);
    return wrapper;
  }

  function attachRecorder(wrapper, input, label, shortcut = false, allowMouse = true, mouseReason = '') {
    const button = element('button', 'record-input quiet', 'Record input…');
    button.type = 'button';
    button.dataset.field = 'record';
    button.setAttribute('aria-label', `Record ${label.toLowerCase()}`);
    const status = element('span', 'record-status');
    status.dataset.field = 'record-status';
    status.setAttribute('role', 'status');
    status.setAttribute('aria-live', 'polite');
    button.addEventListener('click', () => {
      button.textContent = 'Listening…';
      button.setAttribute('aria-pressed', 'true');
      input.readOnly = true;
      recorder.start({input, shortcut, allowMouse, mouseReason, onChange: changed,
        onStatus: text => {if (status.textContent !== text) status.textContent = text;},
        onStop: () => {
          button.textContent = 'Record input…';
          button.setAttribute('aria-pressed', 'false');
          input.readOnly = false;
        }
      });
    });
    wrapper.append(button, status);
  }

  function keyInput(label, value = '', shortcut = false, allowMouse = true, mouseReason = '') {
    const wrapper = element('div', 'binding-field');
    const field = rowInput(label, value);
    wrapper.append(field);
    attachRecorder(wrapper, field.querySelector('input'), label, shortcut, allowMouse, mouseReason);
    return wrapper;
  }

  function controlPicker(source) {
    const wrapper = element('div', 'control-picker');
    const searchLabel = rowInput('Search default key or action');
    const search = searchLabel.querySelector('input');
    search.type = 'search';
    search.dataset.field = 'control-search';
    const label = element('label');
    label.append(element('span', 'sr-only', 'Minecraft control (default key — action)'));
    const select = element('select');
    select.setAttribute('aria-label', 'Minecraft control (default key — action)');
    label.append(select);
    const customLabel = rowInput('Custom control ID', source);
    customLabel.className = 'control-custom';
    const custom = customLabel.querySelector('input');
    custom.dataset.field = 'custom-control';
    const known = MINECRAFT_CONTROLS.some(control => control.id === source);
    let selected = known || !source ? source : '__custom__';
    if (known) custom.value = '';

    function showOptions() {
      const matches = findMinecraftControls(search.value);
      const options = [...matches];
      const current = MINECRAFT_CONTROLS.find(control => control.id === selected);
      // Filtering must not silently discard a selection from a loaded profile.
      if (current && !options.includes(current)) options.unshift(current);
      select.replaceChildren();
      const blank = element('option', '', 'Select a control…');
      blank.value = '';
      select.append(blank);
      for (const control of options) {
        const option = element('option', '', `${control.key} — ${control.action}`);
        option.value = control.id;
        select.append(option);
      }
      if (!matches.length) {
        const empty = element('option', '', 'No matching controls');
        empty.disabled = true;
        empty.value = '__no_matches__';
        select.append(empty);
      }
      const other = element('option', '', 'Custom control ID…');
      other.value = '__custom__';
      select.append(other);
      select.value = selected;
      customLabel.hidden = selected !== '__custom__';
      custom.disabled = customLabel.hidden;
    }

    search.addEventListener('input', event => {
      event.stopPropagation();
      showOptions();
    });
    // Native selects also emit input; handle the selection once on change.
    select.addEventListener('input', event => event.stopPropagation());
    select.addEventListener('change', () => {
      selected = select.value;
      showOptions();
      changed();
    });
    showOptions();
    wrapper.append(searchLabel, label, customLabel);
    return wrapper;
  }

  const mouseSourceHelp = 'Mouse triggers require explicit keyd mouse-device setup. Press a keyboard key.';

  function addMappingRow(section, source = '', target = '', silent = false) {
    const row = element('div', 'mapping-row');
    row.append(section === 'controls' ? controlPicker(source) : keyInput(fields[section][0], source, false, false, mouseSourceHelp),
      keyInput(fields[section][1], target, section === 'ninjabrain'));
    const remove = element('button', 'remove', 'Remove');
    remove.type = 'button';
    remove.setAttribute('aria-label', `Remove ${section} row`);
    remove.addEventListener('click', () => { recorder.stop(); row.remove(); changed(); });
    row.append(remove);
    $(`${section}-rows`).append(row);
    if (!silent) changed();
  }

  function addPresetRow(preset = {}, silent = false) {
    const row = element('div', 'preset-row');
    for (const [key, label, type] of [
      ['id', 'Preset name', 'text'], ['width', 'Width', 'number'],
      ['height', 'Height', 'number'], ['shortcut', 'Shortcut', 'text']
    ]) {
      const wrapper = key === 'shortcut'
        ? keyInput(label, preset[key] ?? '', true, false) : rowInput(label, preset[key] ?? '');
      const input = wrapper.querySelector('input');
      input.dataset.field = key;
      input.type = type;
      input.required = true;
      if (type === 'number') { input.min = '100'; input.max = '16384'; }
      row.append(wrapper);
    }
    const eye = element('label', 'eye-label');
    const check = element('input');
    check.type = 'checkbox';
    check.dataset.field = 'eye_zoom';
    check.checked = Boolean(preset.eye_zoom);
    eye.append(check, document.createTextNode(' Eye zoom'));
    row.append(eye);
    const remove = element('button', 'remove', 'Remove');
    remove.type = 'button';
    remove.setAttribute('aria-label', 'Remove preset');
    remove.addEventListener('click', () => { recorder.stop(); row.remove(); changed(); });
    row.append(remove);
    $('presets-rows').append(row);
    if (!silent) changed();
  }

  function readRows(section) {
    return [...$(`${section}-rows`).children].map(row => {
      if (section !== 'controls') return [...row.querySelectorAll('input')].map(input => input.value);
      const select = row.querySelector('select');
      const source = select.value === '__custom__'
        ? row.querySelector('[data-field="custom-control"]').value : select.value;
      return [source, row.children[1].querySelector('input').value];
    });
  }

  function readProfile() {
    const chat = {};
    for (const name of ['open_chat', 'open_command', 'send', 'cancel']) {
      chat[name] = document.querySelector(`[name="${name}"]`).value.trim();
      if (!chat[name]) throw new Error('Complete all chat keys.');
    }
    const presets = [...$('presets-rows').children].map(row => {
      const get = key => row.querySelector(`[data-field="${key}"]`);
      const width = Number(get('width').value);
      const height = Number(get('height').value);
      if (!get('id').value.trim() || !get('shortcut').value.trim() ||
          !Number.isInteger(width) || !Number.isInteger(height) ||
          width < 100 || height < 100 || width > 16384 || height > 16384) {
        throw new Error('Each preset needs a name, shortcut, and dimensions from 100 to 16384.');
      }
      return {
        id: get('id').value.trim(), width, height,
        shortcut: get('shortcut').value.trim(), eye_zoom: get('eye_zoom').checked
      };
    });
    if (presets.length > 8) throw new Error('Use at most eight presets.');
    return {
      controls: profileFromRows(readRows('controls')),
      crafting: profileFromRows(readRows('crafting')),
      ninjabrain: profileFromRows(readRows('ninjabrain')),
      snippets: $('snippets').value.split(/\r?\n/).map(s => s.trim()).filter(Boolean),
      chat, presets,
      launcher_root: $('launcher-root').value.trim() || null
    };
  }

  function fillProfile(profile) {
    recorder.stop();
    state.savedProfile = profile;
    for (const section of Object.keys(fields)) {
      $(`${section}-rows`).replaceChildren();
      for (const [source, target] of Object.entries(profile[section] || {})) {
        addMappingRow(section, source, target, true);
      }
    }
    $('snippets').value = (profile.snippets || []).join('\n');
    $('launcher-root').value = profile.launcher_root || '';
    for (const name of ['open_chat', 'open_command', 'send', 'cancel']) {
      document.querySelector(`[name="${name}"]`).value = profile.chat?.[name] || '';
    }
    $('presets-rows').replaceChildren();
    for (const preset of profile.presets || []) addPresetRow(preset, true);
    state.revision = 0;
    state.savedRevision = 0;
    state.preview = null;
    updateApply();
  }

  async function api(path, method = 'GET', data) {
    const headers = {};
    if (method !== 'GET') {
      headers.Origin = window.location.origin;
      headers['X-MCSR-Session'] = state.token;
      if (data !== undefined) headers['Content-Type'] = 'application/json';
    }
    const response = await fetch(path, {
      method, headers, credentials: 'same-origin',
      body: data === undefined ? undefined : JSON.stringify(data)
    });
    const body = await response.json();
    if (!response.ok) {
      const detail = body.detail;
      let explanation;
      if (Array.isArray(detail)) explanation = detail.map(item => item.msg).join('; ');
      else explanation = detail?.message || detail?.code || String(detail || response.statusText);
      throw new Error(explanation);
    }
    return body;
  }

  function showStatus(status) {
    state.status = status;
    const grid = $('status');
    grid.replaceChildren();
    const cards = [
      ['Activation', status.live_activation_enabled ? 'Available' : 'Observe only'],
      ['Launcher root', status.launcher_root || 'Not detected'],
      ['Instances', String(status.instances?.length ?? 0)],
      ['Focus', status.focus?.mode || 'Unknown']
    ];
    if (status.focus?.window_mode === 'full_size') {
      cards.push(['Last resize', 'Full-size borderless']);
    } else if (status.focus?.window_mode === 'preset') {
      cards.push(['Last resize', status.focus.active_preset ? `Preset: ${status.focus.active_preset}` : 'Preset size']);
    }
    for (const [label, value] of cards) {
      const card = element('div', 'status-card');
      card.append(element('span', 'status-label', label), element('strong', '', value));
      grid.append(card);
    }
    if (status.instances?.length) {
      const card = element('div', 'status-detail');
      card.append(element('h3', '', 'Discovered instances'));
      renderInstances(card, status.instances.map(item => ({
        instance_id: item.id,
        status: item.problem ? 'failed' : 'detected',
        detail: item.problem || ''
      })));
      grid.append(card);
    }
    const focus = status.focus || {};
    const focusDetails = [
      ['Minecraft focus', focus.minecraft_focused],
      ['GNOME extension', focus.extension_connected],
      ['Activation suspended', focus.activation_suspended]
    ].filter(([, value]) => typeof value === 'boolean');
    if (focusDetails.length) {
      const card = element('div', 'status-detail');
      card.append(element('h3', '', 'Focus service'));
      for (const [label, value] of focusDetails) {
        card.append(element('p', 'help', `${label}: ${value ? 'Yes' : 'No'}`));
      }
      grid.append(card);
    }
    if (focus.cleanup_error) grid.append(element('p', 'message error status-detail', focus.cleanup_error));
    message('cleanup-warning', status.focus?.unsafe_stop_failure ||
      status.focus?.recovery_clear_pending || status.focus?.mutation_recovery_pending
      ? 'Keyd cleanup needs recovery. Check the service status before activating mappings.'
      : '', 'error');
    if (status.focus?.reconciliation) {
      const card = element('div', 'status-detail');
      card.append(element('h3', '', 'Instance reconciliation'));
      renderInstances(card, status.focus.reconciliation.instances || []);
      grid.append(card);
    }
    updateApply();
  }

  function renderInstances(container, instances) {
    const list = element('ul', 'instance-list');
    for (const item of instances) {
      const entry = element('li');
      entry.append(element('strong', '', item.instance_id || 'Unknown instance'));
      entry.append(element('span', `badge ${item.status}`, item.status || 'unknown'));
      if (item.detail) entry.append(element('span', 'detail', item.detail));
      list.append(entry);
    }
    if (!instances.length) container.append(element('p', 'help', 'No instances reported.'));
    else container.append(list);
  }

  function renderPreview(preview) {
    const box = $('preview-results');
    box.replaceChildren(element('h3', '', 'Per-instance preview'));
    renderInstances(box, preview.instances || []);
    if (preview.mappings?.length) {
      const table = element('table');
      const caption = element('caption', '', 'Control mappings by instance');
      const head = element('thead');
      const headRow = element('tr');
      for (const label of ['Instance', 'Physical', 'Emitted', 'Minecraft action']) {
        headRow.append(element('th', '', label));
      }
      head.append(headRow);
      const body = element('tbody');
      for (const mapping of preview.mappings) {
        const row = element('tr');
        for (const value of [mapping.instance_id, mapping.physical, mapping.emitted, mapping.action]) {
          row.append(element('td', '', value ?? ''));
        }
        body.append(row);
      }
      table.append(caption, head, body);
      box.append(table);
    }
    if (preview.instances?.some(item => !['ready', 'unchanged'].includes(item.status))) {
      box.append(element('p', 'message error', 'Apply is blocked until every instance is ready or unchanged.'));
    }
  }

  function renderJob(target, result) {
    const box = $(target);
    box.replaceChildren();
    if (result.error) box.append(element('p', 'message error', result.error));
    if (result.instances) renderInstances(box, result.instances);
    if (result.keyd_error) box.append(element('p', 'message error', `keyd: ${result.keyd_error}`));
    if (result.backup_id) {
      box.append(element('p', 'message', `Backup ID: ${result.backup_id}`));
      if (target === 'apply-results') $('backup-id').value = result.backup_id;
    }
    if (result.version || result.path) {
      box.append(element('p', 'message', `Version: ${result.version || 'unknown'} · Saved to: ${result.path || 'unknown'}`));
    }
  }

  async function runJob(path, target, data) {
    const accepted = await api(path, 'POST', data);
    message(target, `Job ${accepted.id} is running…`);
    for (;;) {
      await new Promise(resolve => setTimeout(resolve, 700));
      const job = await api(`/api/jobs/${encodeURIComponent(accepted.id)}`);
      if (job.status === 'running') continue;
      renderJob(target, job.status === 'failed' ? { error: job.error || 'Job failed.' } : job.result || {});
      await refreshStatus();
      return job;
    }
  }

  async function refreshStatus() { showStatus(await api('/api/status')); }

  async function action(work, errorTarget = 'notice') {
    if (state.busy) return;
    state.busy = true;
    updateApply();
    try { await work(); }
    catch (error) { message(errorTarget, error.message, 'error'); }
    finally { state.busy = false; updateApply(); }
  }

  async function init() {
    try {
      state.token = (await api('/api/session')).token;
      const [profile, status] = await Promise.all([api('/api/profile'), api('/api/status')]);
      fillProfile(profile);
      showStatus(status);
    } catch (error) { message('notice', `Could not load toolkit: ${error.message}`, 'error'); }
  }

  for (const [name, label] of [['open_chat', 'Open chat'], ['open_command', 'Open command'],
    ['send', 'Send'], ['cancel', 'Cancel']]) {
    const input = document.querySelector(`[name="${name}"]`);
    attachRecorder(input.parentElement, input, label, false, false, mouseSourceHelp);
  }
  $('profile-form').addEventListener('submit', event => event.preventDefault());
  $('profile-form').addEventListener('input', changed);
  $('profile-form').addEventListener('change', event => {
    if (event.target.type === 'checkbox') changed();
  });
  $('launcher-root').addEventListener('input', changed);
  for (const button of document.querySelectorAll('[data-add]')) {
    button.addEventListener('click', () => {
      if (button.dataset.add === 'presets') addPresetRow();
      else addMappingRow(button.dataset.add);
    });
  }
  $('refresh-status').addEventListener('click', () => action(refreshStatus));
  $('save-profile').addEventListener('click', () => action(async () => {
    const revision = state.revision;
    const saved = await api('/api/profile', 'PUT', readProfile());
    state.savedProfile = saved;
    state.savedRevision = revision;
    state.preview = null;
    $('preview-results').replaceChildren();
    message('notice', 'Profile saved. Preview changes before applying.');
    await refreshStatus();
  }));
  $('preview-profile').addEventListener('click', () => action(async () => {
    const revision = state.revision;
    state.preview = null;
    updateApply();
    const preview = await api('/api/preview', 'POST', readProfile());
    state.preview = revision === state.revision ? preview : null;
    if (state.preview) renderPreview(preview);
    if (state.savedRevision !== state.revision) message('notice', 'Preview is for an unsaved draft. Save this profile before Apply.');
  }, 'preview-results'));
  $('apply-profile').addEventListener('click', () => action(async () => {
    if (!canApply(state.status, state.preview, state.savedRevision, state.revision, false)) return;
    let job;
    try { job = await runJob('/api/apply', 'apply-results', state.savedProfile); }
    catch (error) {
      state.preview = null;
      $('preview-results').replaceChildren();
      throw error;
    }
    state.preview = null;
    updateApply();
    if (job.status === 'succeeded') message('notice', 'Apply job finished. Review each instance result.');
  }, 'apply-results'));
  $('restore-backup').addEventListener('click', () => action(async () => {
    const backupId = $('backup-id').value.trim();
    if (!backupId) throw new Error('Enter a backup ID.');
    const revision = state.revision;
    const wasSaved = state.savedRevision === revision;
    const job = await runJob('/api/restore', 'restore-results', { backup_id: backupId });
    state.preview = null;
    $('preview-results').replaceChildren();
    if (job.status === 'succeeded') {
      const restored = await api('/api/profile');
      if (wasSaved && state.revision === revision) {
        fillProfile(restored);
        message('notice', 'Restored the saved profile. Preview changes before applying.');
      } else {
        state.savedProfile = restored;
        state.savedRevision = -1;
        message('notice', 'Restore finished. Your unsaved draft was preserved; save and preview before Apply.');
      }
    }
  }, 'restore-results'));
  for (const button of document.querySelectorAll('[data-download]')) {
    button.addEventListener('click', () => action(async () => {
      await runJob(`/api/download/${button.dataset.download}`, `download-${button.dataset.download}-results`);
    }, `download-${button.dataset.download}-results`));
  }
  init();
}
