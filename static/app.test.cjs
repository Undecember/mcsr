const { test } = require('node:test');
const assert = require('node:assert/strict');
const { canApply, profileFromRows } = require('./app.js');

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
