const { test } = require('node:test');
const assert = require('node:assert/strict');
const { canApply, profileFromRows, findMinecraftControls } = require('./app.js');

test('apply requires a saved, current preview with every instance ready or unchanged', () => {
  const safe = { instances: [{ instance_id: 'one', status: 'ready', detail: '' }] };
  assert.equal(canApply({ live_activation_enabled: true }, safe, 2, 2, false), true);
  assert.equal(canApply({ live_activation_enabled: false }, safe, 2, 2, false), false);
  assert.equal(canApply({ live_activation_enabled: true }, safe, 1, 2, false), false);
  assert.equal(canApply({ live_activation_enabled: true }, safe, 2, 2, true), false);
  assert.equal(canApply({ live_activation_enabled: true }, null, 2, 2, false), false);
  assert.equal(canApply({ live_activation_enabled: true }, { instances: [] }, 2, 2, false), false);
  for (const status of ['failed', 'deferred', 'blocked']) {
    assert.equal(canApply({ live_activation_enabled: true }, {
      instances: [...safe.instances, { instance_id: 'two', status, detail: '' }]
    }, 2, 2, false), false);
  }
});

test('profile rows become API dictionaries without blank rows', () => {
  assert.deepEqual(profileFromRows([['key_key.back', 's'], ['', ''], ['key_key.jump', 'space']]), {
    'key_key.back': 's', 'key_key.jump': 'space'
  });
});

test('duplicate profile row keys are rejected before sending', () => {
  assert.throws(() => profileFromRows([['s', 'k'], ['s', 'j']]), /Duplicate source/);
});

// Catch confusing a default key with the stored Minecraft action or matching IDs only.
test('control search finds Minecraft actions by default key or action name', () => {
  assert.equal(typeof findMinecraftControls, 'function');
  assert.deepEqual(findMinecraftControls(' W '), [
    {id: 'key_key.forward', key: 'W', action: 'Move forward'}
  ]);
  assert.deepEqual(findMinecraftControls('hotbar slot 1'), [
    {id: 'key_key.hotbar.1', key: '1', action: 'Hotbar slot 1'}
  ]);
  assert.equal(findMinecraftControls('1')[0].id, 'key_key.hotbar.1');
  assert.equal(findMinecraftControls('space')[0].id, 'key_key.jump');
  assert.equal(findMinecraftControls('right mouse')[0].id, 'key_key.use');
  assert.equal(findMinecraftControls('unbound').some(control => control.id === 'key_key.smoothCamera'), true);
  assert.equal(findMinecraftControls('SNEAK')[0].key, 'Left Shift');
  assert.equal(findMinecraftControls('not a control').length, 0);
  assert.equal(new Set(findMinecraftControls('').map(control => control.id)).size,
    findMinecraftControls('').length);
});

const { keyboardKey, createInputRecorder } = require('./app.js');

// Wrong physical-code translation would silently save a different runtime binding.
test('physical keyboard codes become backend names, including punctuation and sided modifiers', () => {
  for (const [code, name] of [
    ['KeyQ', 'q'], ['Digit0', '0'], ['Backspace', 'backspace'], ['F24', 'f24'],
    ['Escape', 'esc'], ['Tab', 'tab'], ['Space', 'space'], ['ArrowLeft', 'left'],
    ['ControlRight', 'rightcontrol'], ['ShiftLeft', 'leftshift'], ['AltRight', 'rightalt'],
    ['MetaLeft', 'leftmeta'], ['BracketLeft', 'leftbrace'], ['Period', 'dot'],
    ['Numpad7', 'kp7'], ['NumpadEnter', 'kpenter'], ['PrintScreen', 'sysrq']
  ]) assert.equal(keyboardKey(code), name, code);
  for (const code of ['Unidentified', 'Fn', '', 'F25']) assert.equal(keyboardKey(code), null);
});

function recording() {
  const root = new EventTarget();
  const view = new EventTarget();
  let time = 0;
  const intervals = new Set();
  const clock = {now: () => time, setInterval: fn => {intervals.add(fn); return fn;},
    clearInterval: fn => intervals.delete(fn)};
  const recorder = createInputRecorder(root, view, clock);
  const input = {value: 'old', isConnected: true};
  let changed = 0, status = '';
  const start = (options = {}) => recorder.start({input, onChange: () => changed++,
    onStatus: text => {status = text;}, ...options});
  const emit = (type, properties = {}, target = root) => {
    const event = new Event(type, {cancelable: true});
    Object.assign(event, properties);
    target.dispatchEvent(event);
    return event;
  };
  return {input, start, emit, recorder, view, changes: () => changed, status: () => status,
    advance: ms => {time += ms; for (const fn of [...intervals]) fn();}};
}

test('capture records one key, suppresses browser actions and invalidates only changed values', () => {
  const r = recording();
  r.start();
  assert.match(r.status(), /5s/);
  assert.equal(r.emit('keydown', {code: 'Backspace'}).defaultPrevented, true);
  assert.equal(r.input.value, 'backspace');
  assert.equal(r.changes(), 1);
  r.emit('keydown', {code: 'KeyX'});
  assert.equal(r.input.value, 'backspace');
  r.start();
  r.emit('keydown', {code: 'Backspace'});
  assert.equal(r.changes(), 1, 'same binding must preserve a valid preview');
});

test('timeout, blur and detachment preserve the previous binding', () => {
  for (const end of ['timeout', 'blur', 'detach']) {
    const r = recording(); r.start();
    r.emit('keydown', {code: 'Unidentified'});
    r.emit('keydown', {code: 'KeyX', repeat: true});
    r.emit('keydown', {code: 'KeyX', isComposing: true});
    if (end === 'timeout') r.advance(5000);
    if (end === 'blur') r.emit('blur', {}, r.view);
    if (end === 'detach') {r.input.isConnected = false; r.advance(100);}
    r.emit('keydown', {code: 'KeyQ'});
    assert.equal(r.input.value, 'old', end);
    assert.equal(r.changes(), 0, end);
  }
});

test('shortcuts retain modifier sides and forget released modifiers', () => {
  const r = recording(); r.start({shortcut: true});
  r.emit('keydown', {code: 'ControlRight', ctrlKey: true});
  r.emit('keydown', {code: 'ShiftLeft', ctrlKey: true, shiftKey: true});
  r.emit('keyup', {code: 'ShiftLeft', ctrlKey: true});
  assert.equal(r.input.value, 'old', 'modifier alone is not a shortcut');
  r.emit('keydown', {code: 'F6', ctrlKey: true});
  assert.equal(r.input.value, 'rightcontrol+f6');
  const held = recording(); held.start({shortcut: true});
  held.emit('keydown', {code: 'KeyN', ctrlKey: true, altKey: true});
  assert.equal(held.input.value, 'leftcontrol+leftalt+n');
});

test('Escape and Tab are bindings, and a new capture stops the old one', () => {
  const r = recording(); r.start(); r.start();
  r.emit('keydown', {code: 'Escape'});
  assert.equal(r.input.value, 'esc'); assert.equal(r.changes(), 1);
  r.start(); r.emit('keydown', {code: 'Tab'});
  assert.equal(r.input.value, 'tab');
});

test('mouse capture suppresses the entire gesture and leaves subsequent clicks alone', () => {
  const r = recording(); r.start({allowMouse: true});
  r.emit('pointerdown', {button: 0});
  r.emit('mousedown', {button: 0});
  assert.equal(r.input.value, 'leftmouse');
  for (const type of ['pointerup', 'mouseup', 'click']) {
    assert.equal(r.emit(type, {button: 0}).defaultPrevented, true, type);
  }
  assert.equal(r.emit('mousedown', {button: 0}).defaultPrevented, false);
  assert.equal(r.emit('click', {button: 0}).defaultPrevented, false);
});

test('unsupported mouse destinations and buttons never change the binding', () => {
  const r = recording(); r.start({allowMouse: false, shortcut: true});
  assert.equal(r.emit('mousedown', {button: 3}).defaultPrevented, true);
  assert.equal(r.input.value, 'old');
  r.advance(5000);
  assert.equal(r.changes(), 0);
  r.start({allowMouse: true}); r.emit('mousedown', {button: 7});
  assert.equal(r.input.value, 'old');
});


test('holding a recorded button does not leak its release click after a delay', () => {
  const r = recording(); r.start({allowMouse: true});
  r.emit('mousedown', {button: 2});
  r.advance(2000);
  assert.equal(r.emit('contextmenu', {button: 2}).defaultPrevented, true);
  assert.equal(r.emit('mouseup', {button: 2}).defaultPrevented, true);
  assert.equal(r.emit('auxclick', {button: 2}).defaultPrevented, true);
});

test('held recorded keyboard keys do not repeat browser actions after capture', () => {
  const r = recording(); r.start();
  r.emit('keydown', {code: 'F5'}); r.advance(2000);
  assert.equal(r.emit('keydown', {code: 'F5', repeat: true}).defaultPrevented, true);
  assert.equal(r.emit('keyup', {code: 'F5'}).defaultPrevented, true);
  assert.equal(r.emit('keydown', {code: 'F5'}).defaultPrevented, false);
});

test('rejected mouse gestures stay suppressed across recording timeout', () => {
  const r = recording(); r.start({allowMouse: false, shortcut: true});
  assert.equal(r.emit('mousedown', {button: 0}).defaultPrevented, true);
  r.advance(5001);
  assert.equal(r.emit('mouseup', {button: 0}).defaultPrevented, true);
  assert.equal(r.emit('click', {button: 0}).defaultPrevented, true);
  assert.equal(r.input.value, 'old');
});
