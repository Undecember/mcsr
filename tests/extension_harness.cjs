const fs = require('node:fs');
const vm = require('node:vm');
const assert = require('node:assert/strict');
const source = fs.readFileSync('extension/extension.js', 'utf8')
    .replace(/^import .*;$/gm, '')
    .replace('export default class McsrToolkitExtension', 'globalThis.Toolkit = class McsrToolkitExtension');
let modal = 0;
let focused = null;
let rect = {x: 10, y: 20, width: 854, height: 517};
let requested = null;
const callbacks = new Map();
let nextTimer = 1;
let now = 0;
const tick = (ms = 25) => {
    now += ms * 1000;
    for (const [id, callback] of [...callbacks]) {
        if (callbacks.has(id) && callback() === false) callbacks.delete(id);
    }
};
let processes = [];
let tool = '/usr/bin/xprop';
let spawnError = false;
const makeWindow = (options = {}) => {
    const w = {get_wm_class: () => 'Minecraft', decorated: false, xwindow: 0x1234,
        fullscreen: false, maximized: 0, frame: {...rect}, content: {...rect},
        moves: [], get_monitor: () => 0, get_client_type: () => 1, get_property(name, value) {assert.equal(name, 'xwindow'); value.value = this.xwindow;}, get_frame_rect() {return this.frame;},
        get_client_content_rect() {return this.content;}, is_fullscreen() {return this.fullscreen;},
        get_maximize_flags() {return this.maximized;}, unmake_fullscreen() {this.fullscreen = false;},
        unmaximize() {this.maximized = 0;},
        move_resize_frame(_u, x, y, width, height) {this.moves.push({x, y, width, height});},
        ...options};
    return w;
};
const reply = () => ({values: [], errors: [], return_value(v) {this.values.push(Array.from(v.value));},
    return_dbus_error(_name, message) {this.errors.push(message);}});
const applySize = w => {w.frame = {...w.moves.at(-1)}; w.content = {...w.frame};};
const bindings = new Map();
const settings = new Map();
const emitted = [];
const generation = 'a'.repeat(64);
const configuration = (presets, gen = generation) => JSON.stringify({protocol: 'mcsr-keyd-v1', generation: gen, presets});
let grabFailure = null;
let addFailure = null;
let setFailure = null;
const GLib = {PRIORITY_DEFAULT: 0, SOURCE_CONTINUE: true, SOURCE_REMOVE: false,
    timeout_add: (_p, _ms, fn) => { const id = nextTimer++; callbacks.set(id, fn); return id; },
    get_monotonic_time: () => now, find_program_in_path: () => tool,
    source_remove: id => callbacks.delete(id), Variant: class {constructor(type, value) {
        this.type = type; this.value = value;
    }}};
const context = {GLib, Gio: {}, GObject: {TYPE_ULONG: 1, Value: class {init() {} get_ulong() {return this.value;}}}, TextEncoder, Meta: {WindowClientType: {X11: 1, WAYLAND: 2}, MaximizeFlags: {HORIZONTAL: 1, VERTICAL: 2}, KeyBindingFlags: {NONE: 0}, KeyBindingAction: {NONE: 0}},
    Shell: {ActionMode: {NORMAL: 1}}, Extension: class {},
    Main: {layoutManager: {monitors: [{x: 0, y: 0, width: 1920, height: 1080}]}, get modalCount() {return modal;}, wm: {
        addKeybinding: (name, _s, _f, _m, fn) => {
            if (addFailure === name) {addFailure = null; throw Error('mid-add failure');}
            if (grabFailure === name) {grabFailure = null; return 0;}
            bindings.set(name, fn); return 42;
        },
        removeKeybinding: name => bindings.delete(name)}},
    global: {display: {get focus_window() {
        if (focused) focused.get_title ??= () => '';
        return focused;
    }}}, log: () => {}};
context.Gio.SubprocessFlags = {NONE: 0};
context.Gio.Subprocess = {new: argv => {
    if (spawnError) throw Error('spawn failed');
    const p = {argv: Array.from(argv), killed: false, wait_check_async(_c, cb) {this.callback = cb;},
        wait_check_finish() {if (this.failure) throw Error('exit failed'); return true;},
        force_exit() {this.killed = true;}, complete(failure = false) {this.failure = failure; this.callback(this, {});}};
    processes.push(p); return p;
}};
vm.createContext(context);
vm.runInContext(source, context);
const extension = new context.Toolkit();
extension._settings = {
    get_strv: name => settings.get(name) ?? [],
    set_strv: (name, value) => {
        if (setFailure === name) {setFailure = null; throw Error('settings failure');}
        settings.set(name, value); return true;
    }};
extension._bindings = [];
extension._presets = [];
extension._resizeRequests = new Map();
extension._dbus = {emit_signal: (name, value) => emitted.push([name, value.type, value.value])};
extension.ConfigurePresets(configuration([
    {id: 'normal', accelerator: 'F13', eye_zoom: false},
    {id: 'eye', accelerator: 'F14', eye_zoom: true},
]));
assert.deepEqual(Array.from(settings.get('eye-zoom-shift')), ['<Shift>F14']);
assert.equal(bindings.size, 0, 'no global grabs while Minecraft is unfocused');
focused = makeWindow({get_wm_class: () => 'Minecraft 1.16.1', move_resize_frame(_u, x, y, width, height) {requested = {x, y, width, height};}});
extension._sendFocus();
bindings.get('preset-2')();
assert.equal(emitted[0][0], 'PresetActivated');
assert.equal(emitted[0][1], '(sb)');
assert.deepEqual(Array.from(emitted[0][2]), [`${generation}:eye`, false]);
bindings.get('eye-zoom-shift')();
assert.deepEqual(Array.from(emitted[1][2]), [`${generation}:eye`, true]);
const staleBinding = bindings.get('preset-1');
modal = 1;
extension._sendFocus();
assert.equal(bindings.size, 0, 'modal must release grabs');
staleBinding();
assert.equal(emitted.length, 2);
modal = 0;
extension._sendFocus();
assert.equal(bindings.size, 3);
// Fullscreen must be exited by Minecraft itself, without compositor mutations.
let fullscreenMutations = 0;
let fullscreenWindow = makeWindow({fullscreen: true, maximized: 1,
    unmake_fullscreen() {fullscreenMutations++;}, unmaximize() {fullscreenMutations++;},
    move_resize_frame() {fullscreenMutations++;}});
focused = fullscreenWindow;
let fullscreenReply = reply();
const fullscreenProcessCount = processes.length;
extension.ResizeFocusedAsync([280, 1000], fullscreenReply);
assert.equal(fullscreenReply.errors.length, 1);
assert.match(fullscreenReply.errors[0], /fullscreen.*F11/i);
assert.equal(fullscreenMutations, 0);
assert.equal(processes.length, fullscreenProcessCount);
assert.equal(extension._resizeRequests.size, 0);
// Thin presets are centered on the window's current monitor.
const centeredWindow = makeWindow(); focused = centeredWindow;
const centeredReply = reply();
extension.ResizeFocusedAsync([280, 1000], centeredReply);
assert.deepEqual(centeredWindow.moves[0], {x: 820, y: 40, width: 280, height: 1000});
applySize(centeredWindow); tick();
assert.deepEqual(centeredReply.values, [[280, 1000]]);
focused = makeWindow({get_wm_class: () => 'Minecraft 1.16.1', move_resize_frame(_u, x, y, width, height) {requested = {x, y, width, height};}});
let returned = null;
let error = null;
const invocation = {return_value: value => {returned = value.value;},
    return_dbus_error: (_name, message) => {error = message;}};
extension.ResizeFocusedAsync([800, 600], invocation);
assert.equal(returned, null, 'must wait for the observed resize');
assert.deepEqual(requested, {x: 560, y: 240, width: 800, height: 600});
focused.frame = {...requested}; focused.content = {...requested};
tick();
assert.deepEqual(Array.from(returned), [800, 600]);
returned = null;
focused = {get_wm_class: () => 'org.gnome.Terminal'};
extension._sendFocus();
assert.equal(bindings.size, 0, 'blur must release grabs');
extension.ResizeFocusedAsync([800, 600], invocation);
assert.match(error, /focused Minecraft/);
assert.equal(returned, null);
// Borderless preparation targets the X11 client and waits for observed state.
callbacks.clear(); processes = []; now = 0;
let w = makeWindow({decorated: true, get_id: () => 999});
focused = w;
let r = reply();
extension.ResizeFocusedAsync([800, 600], r);
assert.equal(processes.length, 1);
assert.deepEqual(processes[0].argv, ['/usr/bin/xprop', '-id', '0x1234', '-f',
    '_MOTIF_WM_HINTS', '32c', '-set', '_MOTIF_WM_HINTS', '2, 0, 0, 0, 0']);
assert.equal(w.moves.length, 0);
processes[0].complete(); tick();
assert.equal(w.moves.length, 0, 'successful xprop alone is not decoration removal');
w.decorated = false; tick();
assert.deepEqual(w.moves[0], {x: 560, y: 240, width: 800, height: 600});
w.frame = {...w.moves[0]}; w.content = {...w.frame, height: 570}; tick();
assert.equal(r.values.length, 0, 'frame alone must not satisfy content dimensions');
w.content = {...w.frame}; tick();
assert.deepEqual(r.values, [[800, 600]]);
assert.equal(extension._resizeRequests.size, 0);
for (const id of [0, -1, 0x100000000, undefined]) {
    focused = makeWindow({decorated: true, xwindow: id}); r = reply();
    const count = processes.length;
    extension.ResizeFocusedAsync([800, 600], r);
    assert.match(r.errors[0], /ID/); assert.equal(processes.length, count);
}
focused = makeWindow({decorated: true, get_client_type: () => 2}); r = reply();
extension.ResizeFocusedAsync([800, 600], r); assert.match(r.errors[0], /Wayland/);
focused = makeWindow({decorated: false, get_client_type: () => 2}); r = reply();
const count = processes.length; extension.ResizeFocusedAsync([800, 600], r);
assert.equal(processes.length, count); applySize(focused); tick();
assert.deepEqual(r.values, [[800, 600]]);
// Each partial maximization settles before hints or resizing.
for (const max of [1, 2, 3]) {
    w = makeWindow({decorated: true, maximized: max,
        unmaximize(...args) {assert.equal(args.length, 0); assert.equal(this.maximized, max);}});
    focused = w; r = reply(); const before = processes.length;
    extension.ResizeFocusedAsync([800, 600], r); tick();
    assert.equal(processes.length, before); assert.equal(w.moves.length, 0);
    w.maximized = 0; tick();
    assert.equal(processes.length, before + 1); processes.at(-1).complete();
    w.decorated = false; tick(); applySize(w); tick(); assert.deepEqual(r.values, [[800, 600]]);
}
// Entering fullscreen while hints are pending cancels before any resize.
callbacks.clear(); now = 0;
fullscreenWindow = makeWindow({decorated: true}); focused = fullscreenWindow; fullscreenReply = reply();
extension.ResizeFocusedAsync([280, 1000], fullscreenReply);
const fullscreenProcess = processes.at(-1);
fullscreenWindow.fullscreen = true; fullscreenProcess.complete(); tick();
assert.equal(fullscreenReply.errors.length, 1); assert.match(fullscreenReply.errors[0], /fullscreen.*F11/i);
assert.equal(fullscreenWindow.moves.length, 0); assert.equal(extension._resizeRequests.size, 0);
// Polling cancels a still-running preparation process on fullscreen entry.
callbacks.clear(); now = 0;
fullscreenWindow = makeWindow({decorated: true}); focused = fullscreenWindow; fullscreenReply = reply();
extension.ResizeFocusedAsync([280, 1000], fullscreenReply);
const pendingFullscreenProcess = processes.at(-1);
fullscreenWindow.fullscreen = true; tick(); pendingFullscreenProcess.complete();
assert.equal(pendingFullscreenProcess.killed, true);
assert.equal(fullscreenReply.errors.length, 1); assert.equal(fullscreenWindow.moves.length, 0);
assert.equal(extension._resizeRequests.size, 0);
// Entering fullscreen while observing a requested size must not report success.
callbacks.clear(); now = 0;
fullscreenWindow = makeWindow(); focused = fullscreenWindow; fullscreenReply = reply();
extension.ResizeFocusedAsync([280, 1000], fullscreenReply);
applySize(fullscreenWindow); fullscreenWindow.fullscreen = true; tick();
assert.equal(fullscreenReply.errors.length, 1); assert.match(fullscreenReply.errors[0], /fullscreen.*F11/i);
assert.equal(fullscreenReply.values.length, 0); assert.equal(fullscreenWindow.moves.length, 1);
assert.equal(extension._resizeRequests.size, 0);
// Failures, hung subprocesses, overlap and stale callbacks never mutate afterward.
for (const failure of ['missing', 'spawn', 'exit', 'hang', 'focus', 'modal', 'disable', 'unmanaged']) {
    callbacks.clear(); now = 0; tool = failure === 'missing' ? null : '/usr/bin/xprop';
    spawnError = failure === 'spawn';
    let unmanaged;
    w = makeWindow({decorated: true, connect: (_signal, cb) => {unmanaged = cb; return 9;}, disconnect: () => {}});
    focused = w; r = reply(); const before = processes.length;
    extension.ResizeFocusedAsync([800, 600], r);
    spawnError = false; tool = '/usr/bin/xprop';
    if (processes.length > before) {
        const p = processes.at(-1); const other = reply();
        extension.ResizeFocusedAsync([900, 700], other); assert.match(other.errors[0], /pending/);
        if (failure === 'exit') p.complete(true);
        if (failure === 'hang') tick(2000);
        if (failure === 'focus') {focused = makeWindow(); tick();}
        if (failure === 'modal') {modal = 1; tick(); modal = 0;}
        if (failure === 'unmanaged') unmanaged();
        if (failure === 'disable') {
            const original = extension._sendEvent; extension._sendEvent = () => {};
            extension._cancel = {cancel() {}}; extension._dbus = {unexport() {}};
            context.global.display.disconnect = () => {};
            context.Main.layoutManager.disconnectObject = () => {};
            extension.disable(); extension._sendEvent = original;
            extension._dbus = {emit_signal: (name, value) => emitted.push([name, value.type, value.value])};
        }
        assert.equal(p.killed, failure !== 'exit');
        p.complete(); tick();
    }
    assert.equal(r.errors.length, 1, failure); assert.equal(r.values.length, 0, failure);
    assert.equal(w.moves.length, 0, failure); assert.equal(extension._resizeRequests.size, 0, failure);
    assert.equal(callbacks.size, 0, `${failure}: no timer remains`);
}
// Focus loss must cancel even if focus returns before the next timer tick.
callbacks.clear(); now = 0; w = makeWindow({decorated: true}); focused = w; r = reply();
extension.ResizeFocusedAsync([800, 600], r);
const staleProcess = processes.at(-1); focused = makeWindow(); extension._sendFocus();
focused = w; staleProcess.complete(); tick();
assert.equal(r.errors.length, 1); assert.equal(w.moves.length, 0);
assert.equal(staleProcess.killed, true); assert.equal(extension._resizeRequests.size, 0);
// Reject hints ignored by the compositor and a client constrained to a wrong size.
for (const rejected of ['hints', 'size']) {
    callbacks.clear(); now = 0; w = makeWindow({decorated: rejected === 'hints'});
    focused = w; r = reply(); extension.ResizeFocusedAsync([800, 600], r);
    if (rejected === 'hints') processes.at(-1).complete();
    tick(2000); assert.match(r.errors[0], /timed out/); assert.equal(r.values.length, 0);
}
// The monitor is chosen when preparation finishes, using its stage coordinates.
callbacks.clear(); now = 0;
context.Main.layoutManager.monitors = [{x: 0, y: 0, width: 1920, height: 1080},
    {x: -1921, y: -200, width: 1921, height: 1081}];
let monitorIndex = 0;
w = makeWindow({decorated: true, get_monitor: () => monitorIndex}); focused = w; r = reply();
extension.ResizeFocusedAsync([280, 1000], r);
monitorIndex = 1; processes.at(-1).complete(); w.decorated = false; tick();
assert.deepEqual(w.moves[0], {x: -1101, y: -160, width: 280, height: 1000});
applySize(w); tick(); assert.deepEqual(r.values, [[280, 1000]]);
// Missing or invalid monitor information fails without a resize.
for (const [index, monitors] of [[-1, []], [5, []], [0, undefined],
    [0, [{x: 0, y: 0, width: 0, height: 1080}]]]) {
    callbacks.clear(); now = 0; context.Main.layoutManager.monitors = monitors;
    w = makeWindow({get_monitor: () => index}); focused = w; r = reply();
    extension.ResizeFocusedAsync([800, 600], r);
    assert.match(r.errors[0], /monitor geometry/);
    assert.equal(w.moves.length, 0); assert.equal(extension._resizeRequests.size, 0);
}
// The compositor must apply the requested center as well as the dimensions.
callbacks.clear(); now = 0;
context.Main.layoutManager.monitors = [{x: 0, y: 0, width: 1920, height: 1080}];
w = makeWindow(); focused = w; r = reply();
extension.ResizeFocusedAsync([280, 1000], r); applySize(w);
w.frame.x += 1; w.content.x += 1; tick();
assert.equal(r.values.length, 0); tick(2000);
assert.match(r.errors[0], /timed out.*requested position 820,40/);
assert.equal(r.values.length, 0);
// Repeated presets center each exact result on the current monitor.
callbacks.clear(); now = 0; w = makeWindow(); focused = w;
for (const [width, height] of [[800, 600], [1920, 100], [300, 300]]) {
    r = reply(); extension.ResizeFocusedAsync([width, height], r); applySize(w); tick();
    assert.deepEqual(r.values, [[width, height]]);
    assert.equal(w.moves.at(-1).x, Math.floor((1920 - width) / 2));
    assert.equal(w.moves.at(-1).y, Math.floor((1080 - height) / 2));
}
focused = null;
console.log('extension behavior checks passed');
// The service owns FIFO creation; a missing path must only schedule a retry.
let opened = false;
extension._cancel = {is_cancelled: () => false};
extension._fifo = {query_info: () => {throw Error('missing');},
    open_readwrite_async: () => {opened = true;}};
context.Gio.FileQueryInfoFlags = {NOFOLLOW_SYMLINKS: 1};
extension._openFifo();
assert.equal(opened, false);
assert.ok(extension._retry);
// A regular file is refused without opening it.
extension._fifo.query_info = () => ({get_attribute_uint32: () => 0o100600});
extension._retry = null;
extension._openFifo();
assert.equal(opened, false);
// Reconfiguration drops stale bindings and an invalid replacement leaves valid ones intact.
extension.ConfigurePresets(configuration(JSON.parse('[{"id":"only","accelerator":"F15","eye_zoom":false}]')));
assert.equal(bindings.size, 0);
assert.throws(() => extension.ConfigurePresets(configuration(JSON.parse('[{"id":"bad","accelerator":"F15","eye_zoom":true},{"id":"bad","accelerator":"F16","eye_zoom":false}]'))));
assert.equal(bindings.size, 0);
// Limits are checked before Mutter is touched, and loss of focus cancels pending observation.
focused = makeWindow({move_resize_frame(_u, x, y, width, height) {requested = {x, y, width, height};}});
error = null;
extension.ResizeFocusedAsync([99, 600], invocation);
assert.match(error, /invalid Minecraft window size/);
error = null;
callbacks.clear();
extension.ResizeFocusedAsync([900, 700], invocation);
focused = null;
tick();
assert.match(error, /focus lost/);
assert.equal(extension._resizeRequests.size, 0);

focused = {get_wm_class: () => 'Minecraft'};
extension._sendFocus();
// GNOME reports failed grabs as NONE; retain the prior working shortcut on failure.
const replacement = configuration(JSON.parse('[{"id":"new","accelerator":"F17","eye_zoom":false},{"id":"next","accelerator":"F18","eye_zoom":false}]'), 'b'.repeat(64));
for (const failure of ['grab', 'add', 'settings']) {
    if (failure === 'grab') grabFailure = 'preset-2';
    if (failure === 'add') addFailure = 'preset-2';
    if (failure === 'settings') setFailure = 'preset-2';
    assert.throws(() => extension.ConfigurePresets(replacement), /grab|mid-add|settings/);
    assert.deepEqual(Array.from(bindings.keys()), ['preset-1']);
    assert.deepEqual(Array.from(settings.get('preset-1')), ['F15']);
    assert.deepEqual(Array.from(settings.get('preset-2')), []);
    focused = {get_wm_class: () => 'Minecraft'};
    bindings.get('preset-1')();
    assert.deepEqual(Array.from(emitted.at(-1)[2]), [`${generation}:only`, false]);
}

// Reject incompatible wire shapes before touching existing bindings.
assert.throws(() => extension.ConfigurePresets('[]'), /protocol|configuration/);
assert.throws(() => extension.ConfigurePresets(JSON.stringify({protocol: 'other', generation, presets: []})), /protocol|configuration/);
const beforeReconfigure = bindings.get('preset-1');
const sameIdGeneration = 'c'.repeat(64);
extension.ConfigurePresets(configuration([{id: 'only', accelerator: 'F15', eye_zoom: false}], sameIdGeneration));
const previousEmissionCount = emitted.length;
beforeReconfigure();
assert.equal(emitted.length, previousEmissionCount, 'old callback cannot emit a new-generation command');
bindings.get('preset-1')();
assert.deepEqual(Array.from(emitted.at(-1)[2]), [`${sameIdGeneration}:only`, false]);

// A path may turn into a regular file during async open. Verify the opened stream.
context.Gio.Credentials = class {get_unix_user() {return 1000;}};
const fileInfo = (mode, uid = 1000, inode = '123') => ({
    has_attribute: () => true,
    get_attribute_uint32: name => name === 'unix::mode' ? mode : uid,
    get_attribute_as_string: name => name === 'unix::inode' ? inode : '46',
});
for (const [mode, uid, pathnameMode, inode] of [
    [0o100600, 1000, 0o010600, '123'], [0o010666, 1000, 0o010600, '123'],
    [0o010600, 1001, 0o010600, '123'], [0o010600, 1000, 0o120777, '123'],
    [0o010600, 1000, 0o010600, '456'],
]) {
    let closed = false;
    const channel = {query_info: () => fileInfo(mode, uid, inode), close: () => {closed = true;},
        get_output_stream: () => {throw Error('must refuse before output');}};
    let queryCount = 0;
    extension._fifo = {query_info: () => fileInfo(queryCount++ ? pathnameMode : 0o010600),
        open_readwrite_async: (_p, _c, cb) => cb(extension._fifo, {}),
        open_readwrite_finish: () => channel};
    extension._retry = null;
    extension._opening = false;
    extension._channel = null;
    extension._openFifo();
    assert.equal(closed, true, 'unsafe opened stream must be closed');
    assert.equal(extension._channel, null);
    assert.ok(extension._retry);
}
// A stable private FIFO is accepted after descriptor and nofollow pathname checks.
const safeInfo = fileInfo(0o010600);
const safeOutput = {};
let initialFocusSent = false;
extension._sendFocus = () => {initialFocusSent = true;};
extension._fifo = {query_info: () => safeInfo,
    open_readwrite_async: (_p, _c, cb) => cb(extension._fifo, {}),
    open_readwrite_finish: () => ({query_info: () => safeInfo,
        get_output_stream: () => safeOutput, close: () => {}})};
extension._retry = null;
extension._openFifo();
assert.equal(extension._pipe, safeOutput);
assert.equal(initialFocusSent, true);
// Async FIFO writes use refcounted GBytes and complete partial writes before a new event.
context.GLib.Bytes = class {constructor(data) {this.data = data;}};
const writes = [];
let pendingWrite = null;
let finishCount = 0;
extension._pipe = {
    write_bytes_async: (bytes, _priority, _cancel, callback) => {
        writes.push(bytes);
        pendingWrite = callback;
    },
    write_bytes_finish: () => finishCount,
};
extension._channel = {close: () => {}};
extension._writing = false;
extension._pendingEvent = null;
extension._sendEvent('Minecraft', 'Game', false);
assert.ok(extension._writingBytes instanceof context.GLib.Bytes);
assert.equal(extension._writingBytes, writes[0]);
assert.ok(pendingWrite);
finishCount = 10;
pendingWrite(extension._pipe, {});
assert.equal(writes.length, 2);
assert.equal(new TextDecoder().decode(writes[1].data),
    new TextDecoder().decode(writes[0].data).slice(10));
finishCount = writes[1].data.length;
pendingWrite(extension._pipe, {});
assert.equal(extension._writingBytes, null);
// Letter shortcuts are physically ungrabbed outside game focus, and focus-time
// installation failure removes every partial grab before the next retry.
focused = null;
extension._presets = [];
extension.ConfigurePresets(configuration(JSON.parse('[{"id":"letter","accelerator":"F19","eye_zoom":false},{"id":"next","accelerator":"F18","eye_zoom":false}]')));
assert.equal(bindings.size, 0);
focused = {get_wm_class: () => 'Minecraft'};
grabFailure = 'preset-2';
extension._syncPresetBindings();
assert.equal(bindings.size, 0);
extension._syncPresetBindings();
assert.deepEqual(Array.from(bindings.keys()), ['preset-1', 'preset-2']);
// Extension loss releases the Mutter grabs even when focus remains Minecraft.
const disabling = new context.Toolkit();
let canceled = false;
let unexported = false;
disabling._sendEvent = () => {};
disabling._bindings = Array.from(bindings.keys());
disabling._resizeRequests = new Map();
disabling._cancel = {cancel: () => {canceled = true;}};
disabling._dbus = {unexport: () => {unexported = true;}};
context.global.display.disconnect = () => {};
context.Main.layoutManager.disconnectObject = () => {};
disabling.disable();
assert.equal(bindings.size, 0);
assert.equal(canceled, true);
assert.equal(unexported, true);

// A grab collision first encountered on focus regain must block producer focus.
focused = null;
extension.ConfigurePresets(configuration([{id: 'late', accelerator: 'F13', eye_zoom: false}]));
focused = makeWindow();
grabFailure = 'preset-1';
context.Toolkit.prototype._sendFocus.call(extension);
assert.equal(bindings.size, 0);
assert.equal(extension.GetFocus()[4], true, 'unavailable resize consumer must report blocked focus');
context.Toolkit.prototype._sendFocus.call(extension);
assert.equal(extension.GetFocus()[4], false, 'successful retry restores consumer readiness');

// New bindings cannot be committed until an ordered keyd marker has drained input.
const nextGeneration = 'c'.repeat(64);
const beforeDrain = bindings.get('preset-1');
extension.PrepareRouting(nextGeneration);
assert.deepEqual([...bindings.keys()], ['routing-fence']);
assert.equal(extension.GetFocus()[4], true);
const pendingConfig = JSON.stringify({protocol:'mcsr-keyd-v1',generation:nextGeneration,drain:true,presets:[{id:'drained',accelerator:'F13',eye_zoom:false}]});
assert.throws(() => extension.ConfigurePresets(pendingConfig), /input drain pending/);
const countBeforeOld = emitted.length;
beforeDrain();
assert.equal(emitted.length, countBeforeOld);
bindings.get('routing-fence')();
assert.equal(extension.GetFocus()[4], true);
extension.ConfigurePresets(pendingConfig);
assert.equal(extension.GetFocus()[4], false);
assert.deepEqual([...bindings.keys()], ['preset-1']);
extension.PrepareRouting('d'.repeat(64));
extension.PrepareRouting('');
assert.equal(bindings.size, 0, 'aborted drain releases the temporary fence grab');

extension.PrepareRouting('e'.repeat(64));
tick(5000);
assert.equal(bindings.size,0,'abandoned input drain expires without a service abort');
