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
