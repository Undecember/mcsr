// Run against a static server: playwright-cli run-code --filename=tests/key_capture_browser.cjs
// The server must serve static/ at http://127.0.0.1:8878; API calls are isolated here.
async function keyCaptureBrowser(page) {
  const check = (condition, message) => {if (!condition) throw new Error(message);};
  let profile = {controls: {'key_key.jump': 'space'}, crafting: {s: 'k'},
    ninjabrain: {q: 'leftcontrol+n'}, snippets: [],
    chat: {open_chat: 't', open_command: 'slash', send: 'enter', cancel: 'esc'},
    presets: [{id: 'small', width: 800, height: 600, shortcut: 'f6'}], launcher_root: null};
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/api/**', async route => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    let body;
    if (path === '/api/session') body = {token: 'test'};
    else if (path === '/api/profile') {
      if (request.method() === 'PUT') profile = request.postDataJSON();
      body = profile;
    } else if (path === '/api/status') body = {live_activation_enabled: true,
      instances: [{id: 'test'}], focus: {mode: 'live'}};
    else if (path === '/api/preview') body = {instances: [{instance_id: 'test', status: 'ready'}]};
    else throw new Error(`Unexpected API ${path}`);
    await route.fulfill({json: body});
  });
  const cdp = await page.context().newCDPSession(page);
  await page.clock.install();
  await page.setViewportSize({width: 1280, height: 900});
  await page.goto('http://127.0.0.1:8878');
  const control = page.locator('#controls-rows .binding-field');
  await control.locator('input').waitFor();
  check(await page.locator('[data-field="record"]').count() === 10, 'Every binding field needs capture');
  check(await page.locator('.control-picker [data-field="record"]').count() === 0, 'No capture for action picker');
  await page.locator('#preview-profile').click();
  await page.waitForFunction(() => !document.getElementById('apply-profile').disabled);
  await control.locator('button').click();
  check(await control.locator('input').inputValue() === 'space', 'Initiating click was recorded');
  await page.keyboard.press('Backspace');
  check(await control.locator('input').inputValue() === 'backspace', 'Backspace capture');
  check(await page.locator('#apply-profile').isDisabled(), 'Capture must invalidate preview');
  const before = await page.locator('#controls-rows .mapping-row').count();
  for (const [button, expected] of [['left', 'leftmouse'], ['middle', 'middlemouse'], ['right', 'rightmouse'],
    ['back', 'mouse1'], ['forward', 'mouse2']]) {
    await control.locator('button').click();
    const bounds = await page.locator('#controls-rows .remove').boundingBox();
    const x = bounds.x + bounds.width / 2, y = bounds.y + bounds.height / 2;
    if (button === 'back' || button === 'forward') {
      await cdp.send('Input.dispatchMouseEvent', {type: 'mousePressed', x, y, button,
        buttons: button === 'back' ? 8 : 16, clickCount: 1});
      await cdp.send('Input.dispatchMouseEvent', {type: 'mouseReleased', x, y, button, buttons: 0, clickCount: 1});
    } else await page.mouse.click(x, y, {button});
    check(await control.locator('input').inputValue() === expected, `${button} capture failed`);
    check(await page.locator('#controls-rows .mapping-row').count() === before, 'Captured click deleted row');
    check(page.url() === 'http://127.0.0.1:8878/', 'Captured side button navigated');
  }
  await control.locator('button').click();
  await page.clock.fastForward(5100);
  check(await control.locator('input').inputValue() === 'mouse2', 'Timeout overwrote binding');
  check((await control.locator('[role="status"]').textContent()).includes('timed out'), 'Timeout status missing');
  await control.locator('button').focus();
  await page.keyboard.press('Enter');
  check(await control.locator('input').inputValue() === 'mouse2', 'Keyboard activation recorded itself');
  await page.keyboard.press('Tab');
  check(await control.locator('input').inputValue() === 'tab', 'Tab binding must be capturable');
  const shortcut = page.locator('#ninjabrain-rows .binding-field').nth(1);
  await shortcut.locator('button').click();
  await page.keyboard.down('ControlRight');
  await page.keyboard.press('F6');
  await page.keyboard.up('ControlRight');
  check(await shortcut.locator('input').inputValue() === 'rightcontrol+f6', 'Sided modifier shortcut');
  const preset = page.locator('#presets-rows .binding-field');
  await preset.locator('button').click();
  await page.mouse.click(20, 20, {button: 'middle'});
  check(await preset.locator('input').inputValue() === 'f6', 'Preset accepted a mouse button');
  await page.keyboard.press('Alt+F8');
  check(await preset.locator('input').inputValue() === 'leftalt+f8', 'Preset keyboard shortcut');
  await preset.locator('button').click();
  const removePreset = await page.locator('#presets-rows .remove').boundingBox();
  await page.mouse.move(removePreset.x + removePreset.width / 2, removePreset.y + removePreset.height / 2);
  await page.mouse.down();
  await page.clock.fastForward(5100);
  await page.mouse.up();
  check(await page.locator('#presets-rows .preset-row').count() === 1,
    'Rejected held mouse gesture clicked Remove after timeout');
  const chat = page.locator('#chat-cancel').locator('..');
  await chat.locator('button').click();
  await page.keyboard.press('Escape');
  check(await chat.locator('input').inputValue() === 'esc', 'Chat Escape capture');
  await page.locator('#save-profile').click();
  await page.waitForFunction(() => document.getElementById('notice').textContent.includes('Profile saved'));
  check(profile.controls['key_key.jump'] === 'tab', 'Profile save lost captured input');
  check(profile.ninjabrain.q === 'rightcontrol+f6', 'Profile save lost shortcut');
  check(profile.presets[0].shortcut === 'leftalt+f8', 'Profile save lost preset');
  await page.setViewportSize({width: 390, height: 844});
  check(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), 'Mobile page overflows');
  check(errors.length === 0, errors.join('\n'));
  return 'Browser checks passed: keyboard, all five mouse buttons, click-through, timeout, shortcuts, save and mobile layout.';
}
