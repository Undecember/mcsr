import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const XML = `<node>
  <interface name="org.mcsr.Toolkit">
    <method name="GetFocus">
      <arg name="class" type="s" direction="out"/>
      <arg name="title" type="s" direction="out"/>
      <arg name="width" type="u" direction="out"/>
      <arg name="height" type="u" direction="out"/>
      <arg name="modal" type="b" direction="out"/>
    </method>
    <method name="ResizeFocused">
      <arg name="width" type="u" direction="in"/>
      <arg name="height" type="u" direction="in"/>
      <arg name="applied_width" type="u" direction="out"/>
      <arg name="applied_height" type="u" direction="out"/>
    </method>
    <method name="ConfigurePresets">
      <arg name="configuration" type="s" direction="in"/>
    </method>
    <signal name="PresetActivated">
      <arg name="preset_id" type="s"/>
      <arg name="shifted" type="b"/>
    </signal>
  </interface>
</node>`;

function minecraftWindow() {
    const window = global.display.focus_window;
    if (!window || Main.modalCount > 0)
        return null;
    return (window.get_wm_class() ?? '').toLowerCase().startsWith('minecraft') ? window : null;
}

export default class McsrToolkitExtension extends Extension {
    enable() {
        this._dbus = Gio.DBusExportedObject.wrapJSObject(XML, this);
        this._dbus.export(Gio.DBus.session, '/org/mcsr/Toolkit');
        this._fifo = Gio.File.new_for_path(`${GLib.get_user_runtime_dir()}/mcsr-toolkit-focus.fifo`);
        this._cancel = new Gio.Cancellable();
        this._focusSignal = global.display.connect('notify::focus-window', () => this._sendFocus());
        Main.layoutManager.connectObject('system-modal-opened', () => this._sendFocus(), this);
        this._lastModal = Main.modalCount > 0;
        this._heartbeat = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 500, () => {
            this._lastModal = Main.modalCount > 0;
            this._sendFocus();
            return GLib.SOURCE_CONTINUE;
        });
        this._openFifo();
        this._bindings = [];
        this._presets = [];
        this._resizeRequests = new Map();
        try {
            this._settings = this.getSettings();
            // Shortcuts stay inactive until the service supplies its current profile.
        } catch (error) {
            log(`MCSR Toolkit shortcuts unavailable: ${error}`);
        }
    }

    disable() {
        this._sendEvent('', '', true);
        for (const request of this._resizeRequests?.values() ?? [])
            this._finishResize(request, 'extension disabled', null, 'org.mcsr.Toolkit.Disabled');
        if (this._retry)
            GLib.source_remove(this._retry);
        for (const name of this._bindings ?? [])
            Main.wm.removeKeybinding(name);
        this._bindings = [];
        global.display.disconnect(this._focusSignal);
        Main.layoutManager.disconnectObject(this);
        GLib.source_remove(this._heartbeat);
        this._cancel.cancel();
        this._channel?.close(null);
        this._channel = null;
        this._pipe = null;
        this._dbus.unexport();
        this._dbus = null;
    }

    ConfigurePresets(configuration) {
        if (!this._settings)
            throw new Error('shortcut settings unavailable');
        if (configuration.length > 8192)
            throw new Error('preset configuration too large');
        const presets = JSON.parse(configuration);
        if (!Array.isArray(presets) || presets.length > 8)
            throw new Error('invalid preset configuration');
        const ids = new Set();
        const accelerators = new Set();
        for (const preset of presets) {
            if (!preset || typeof preset.id !== 'string' ||
                !/^[a-z][a-z0-9_-]{0,31}$/.test(preset.id) || ids.has(preset.id) ||
                typeof preset.accelerator !== 'string' ||
                !/^(<(Control|Shift|Alt|Super)>)*[A-Za-z0-9_]+$/.test(preset.accelerator) ||
                accelerators.has(preset.accelerator) || typeof preset.eye_zoom !== 'boolean')
                throw new Error('invalid preset configuration');
            ids.add(preset.id);
            accelerators.add(preset.accelerator);
        }
        const eyePresets = presets.filter(preset => preset.eye_zoom);
        if (eyePresets.length > 1 || eyePresets.some(preset =>
            preset.accelerator.includes('<Shift>') ||
            accelerators.has(`<Shift>${preset.accelerator}`)))
            throw new Error('invalid eye zoom shortcut');
        const previousPresets = this._presets;
        const names = [...Array.from({length: 8}, (_, index) => `preset-${index + 1}`),
            'eye-zoom-shift'];
        const previousSettings = new Map(names.map(name => [name, this._settings.get_strv(name)]));
        this._removePresetBindings();
        try {
            this._installPresetBindings(presets);
            this._presets = presets;
        } catch (error) {
            this._removePresetBindings();
            try {
                for (const [name, value] of previousSettings) {
                    if (!this._settings.set_strv(name, value))
                        throw new Error(`settings restore failed: ${name}`);
                }
                this._installPresetBindings(previousPresets, false);
                this._presets = previousPresets;
            } catch (restoreError) {
                this._removePresetBindings();
                this._presets = [];
                throw new Error(`${error}; shortcut rollback failed: ${restoreError}`);
            }
            throw error;
        }
    }

    _removePresetBindings() {
        for (const name of this._bindings)
            Main.wm.removeKeybinding(name);
        this._bindings = [];
    }

    _installPresetBindings(presets, updateSettings = true) {
        for (let index = 1; index <= 8; index++) {
            const preset = presets[index - 1];
            const name = `preset-${index}`;
            if (updateSettings && !this._settings.set_strv(name, preset ? [preset.accelerator] : []))
                throw new Error(`settings write failed: ${name}`);
            if (preset && minecraftWindow())
                this._addPresetBinding(name, preset.id, false);
        }
        const eye = presets.find(preset => preset.eye_zoom);
        if (updateSettings && !this._settings.set_strv('eye-zoom-shift',
            eye ? [`<Shift>${eye.accelerator}`] : []))
            throw new Error('settings write failed: eye-zoom-shift');
        if (eye && minecraftWindow())
            this._addPresetBinding('eye-zoom-shift', eye.id, true);
    }

    _syncPresetBindings() {
        // A callback guard cannot release a Mutter grab: remove the binding itself
        // whenever the physical key belongs to another app or a Shell modal.
        if (!minecraftWindow()) {
            this._removePresetBindings();
            return;
        }
        if (!this._settings || this._bindings.length || !this._presets.length)
            return;
        try {
            this._installPresetBindings(this._presets, false);
        } catch (error) {
            this._removePresetBindings();
            log(`MCSR Toolkit shortcuts unavailable: ${error}`);
        }
    }

    _addPresetBinding(name, presetId, shifted) {
        const action = Main.wm.addKeybinding(name, this._settings, Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.NORMAL, () => {
                if (minecraftWindow())
                    this._dbus.emit_signal('PresetActivated',
                        new GLib.Variant('(sb)', [presetId, shifted]));
            });
        if (action === Meta.KeyBindingAction.NONE)
            throw new Error(`shortcut grab failed: ${name}`);
        this._bindings.push(name);
    }

    _retryFifo() {
        if (this._cancel.is_cancelled() || this._retry)
            return;
        this._retry = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 1000, () => {
            this._retry = null;
            this._openFifo();
            return GLib.SOURCE_REMOVE;
        });
    }

    _openFifo() {
        if (this._cancel.is_cancelled() || this._opening || this._channel)
            return;
        let expectedInfo;
        const attributes = 'unix::mode,unix::uid,unix::inode,unix::device';
        try {
            const info = this._fifo.query_info(attributes,
                Gio.FileQueryInfoFlags.NOFOLLOW_SYMLINKS, this._cancel);
            this._validateFifoInfo(info);
            expectedInfo = info;
        } catch (error) {
            this._retryFifo();
            return;
        }
        this._opening = true;
        // Open only an existing path. Unlike append_to_async this cannot create a file.
        this._fifo.open_readwrite_async(GLib.PRIORITY_DEFAULT, this._cancel, (file, result) => {
            this._opening = false;
            let channel;
            try {
                channel = file.open_readwrite_finish(result);
                if (this._cancel.is_cancelled()) {
                    channel.close(null);
                    return;
                }
                // FileIOStream.query_info derives metadata from the opened descriptor.
                const openedInfo = channel.query_info(attributes, this._cancel);
                this._validateFifoInfo(openedInfo);
                const currentInfo = file.query_info(attributes,
                    Gio.FileQueryInfoFlags.NOFOLLOW_SYMLINKS, this._cancel);
                this._validateFifoInfo(currentInfo);
                for (const attribute of ['unix::inode', 'unix::device']) {
                    const identity = openedInfo.get_attribute_as_string(attribute);
                    if (identity !== expectedInfo.get_attribute_as_string(attribute) ||
                        identity !== currentInfo.get_attribute_as_string(attribute))
                        throw new Error('focus FIFO was replaced during open');
                }
                this._channel = channel;
                this._pipe = channel.get_output_stream();
                this._sendFocus();
            } catch (error) {
                channel?.close(null);
                this._channel = null;
                this._pipe = null;
                this._retryFifo();
            }
        });
    }

    _validateFifoInfo(info) {
        for (const attribute of ['unix::mode', 'unix::uid', 'unix::inode', 'unix::device']) {
            if (!info.has_attribute(attribute))
                throw new Error(`missing focus FIFO metadata: ${attribute}`);
        }
        const mode = info.get_attribute_uint32('unix::mode');
        if ((mode & 0o170000) !== 0o010000 || (mode & 0o077) !== 0 ||
            info.get_attribute_uint32('unix::uid') !== new Gio.Credentials().get_unix_user())
            throw new Error('unsafe focus FIFO');
    }

    _sendEvent(cls, title, modal) {
        this._pendingEvent = new TextEncoder().encode(
            `${JSON.stringify({class: cls, title, modal})}\n`);
        this._flushEvent();
    }

    _flushEvent() {
        if (!this._pipe || this._writing || !this._pendingEvent || this._cancel.is_cancelled())
            return;
        const bytes = this._pendingEvent;
        this._pendingEvent = null;
        this._writing = true;
        this._writeEventBytes(bytes, 0);
    }

    _writeEventBytes(bytes, offset) {
        // A refcounted GBytes keeps the backing memory valid for Gio's async operation.
        const remaining = bytes.subarray(offset);
        this._writingBytes = new GLib.Bytes(remaining);
        this._pipe.write_bytes_async(this._writingBytes, GLib.PRIORITY_DEFAULT,
            this._cancel, (stream, result) => {
            try {
                const written = stream.write_bytes_finish(result);
                if (written <= 0 || written > remaining.length)
                    throw new Error('incomplete focus FIFO write');
                if (written < remaining.length) {
                    this._writeEventBytes(bytes, offset + written);
                    return;
                }
                this._writing = false;
                this._writingBytes = null;
                this._flushEvent();
            } catch (error) {
                this._writing = false;
                this._writingBytes = null;
                this._channel?.close(null);
                this._channel = null;
                this._pipe = null;
                this._retryFifo();
            }
        });
    }

    _sendFocus() {
        for (const request of this._resizeRequests?.values() ?? [])
            this._resizeAllowed(request);
        this._syncPresetBindings();
        const window = global.display.focus_window;
        this._sendEvent(window?.get_wm_class() ?? '', window?.get_title() ?? '',
            Main.modalCount > 0);
    }

    GetFocus() {
        const window = global.display.focus_window;
        const rect = window?.get_frame_rect();
        return [window?.get_wm_class() ?? '', window?.get_title() ?? '',
            rect?.width ?? 0, rect?.height ?? 0, Main.modalCount > 0];
    }

    _finishResize(request, error = null, dimensions = null,
        errorName = 'org.mcsr.Toolkit.ResizeFailed') {
        if (request.done)
            return;
        request.done = true;
        if (request.timer)
            GLib.source_remove(request.timer);
        request.timer = 0;
        if (request.process) {
            request.process.force_exit();
            request.process = null;
        }
        if (request.unmanagedSignal)
            request.window.disconnect(request.unmanagedSignal);
        this._resizeRequests.delete(request);
        if (error)
            request.invocation.return_dbus_error(errorName, error);
        else
            request.invocation.return_value(new GLib.Variant('(uu)', dimensions));
    }

    _resizeAllowed(request) {
        if (request.done)
            return false;
        if (minecraftWindow() !== request.window) {
            this._finishResize(request, 'Minecraft focus lost or Shell modal opened during resize');
            return false;
        }
        if (request.window.is_fullscreen()) {
            this._finishResize(request, 'Leave fullscreen using Minecraft’s own fullscreen control (usually F11), then retry');
            return false;
        }
        if (GLib.get_monotonic_time() >= request.deadline) {
            let observed = '';
            try {
                const frame = request.window.get_frame_rect();
                const content = request.window.get_client_content_rect();
                observed = ` (frame ${frame.width}x${frame.height} at ${frame.x},${frame.y}, requested position ${request.x},${request.y}, content ${content.width}x${content.height}, decorated ${request.window.decorated}, fullscreen ${request.window.is_fullscreen()}, maximized ${request.window.get_maximize_flags()})`;
            } catch (_) {}
            this._finishResize(request, `Minecraft borderless resize timed out${observed}`);
            return false;
        }
        return true;
    }

    _prepareBorderless(request) {
        const window = request.window;
        if (!this._resizeAllowed(request))
            return;
        if (!window.decorated) {
            request.prepared = true;
            return;
        }
        if (window.get_client_type() !== Meta.WindowClientType.X11) {
            this._finishResize(request, 'Native Wayland decorations cannot be removed; use client-side borderless support or an XWayland game');
            return;
        }
        // MetaWindowXwayland inherits the runtime X11 subclass property, which
        // is not part of the introspected Meta.Window prototype.
        let xid;
        try {
            const value = new GObject.Value();
            value.init(GObject.TYPE_ULONG);
            window.get_property('xwindow', value);
            xid = value.get_ulong();
        } catch (_) {
            this._finishResize(request, 'Minecraft X11 client window ID is unavailable');
            return;
        }
        if (!Number.isInteger(xid) || xid <= 0 || xid > 0xffffffff) {
            this._finishResize(request, 'Minecraft X11 client window ID is unavailable');
            return;
        }
        const xprop = GLib.find_program_in_path('xprop');
        if (!xprop) {
            this._finishResize(request, 'xprop is required for XWayland borderless resize; install x11-utils');
            return;
        }
        if (!this._resizeAllowed(request))
            return;
        try {
            const process = Gio.Subprocess.new([xprop, '-id', `0x${xid.toString(16)}`,
                '-f', '_MOTIF_WM_HINTS', '32c', '-set', '_MOTIF_WM_HINTS',
                '2, 0, 0, 0, 0'], Gio.SubprocessFlags.NONE);
            request.process = process;
            process.wait_check_async(null, (proc, result) => {
                if (request.done)
                    return;
                request.process = null;
                try {
                    if (!proc.wait_check_finish(result))
                        throw new Error('xprop failed');
                    if (!this._resizeAllowed(request))
                        return;
                    request.prepared = true;
                } catch (error) {
                    this._finishResize(request, `xprop borderless preparation failed: ${error}`);
                }
            });
        } catch (error) {
            this._finishResize(request, `could not start xprop borderless preparation: ${error}`);
        }
    }

    _observeResize(request) {
        if (!this._resizeAllowed(request))
            return GLib.SOURCE_REMOVE;
        const window = request.window;
        try {
            if (!request.statesRequested) {
                request.statesRequested = true;
                const maximized = window.get_maximize_flags();
                if (maximized) {
                    if (!this._resizeAllowed(request)) return GLib.SOURCE_REMOVE;
                    window.unmaximize();
                }
            }
            if (window.get_maximize_flags())
                return GLib.SOURCE_CONTINUE;
            if (!request.preparationStarted) {
                request.preparationStarted = true;
                this._prepareBorderless(request);
            }
            if (request.done)
                return GLib.SOURCE_REMOVE;
            if (!request.prepared || window.decorated)
                return GLib.SOURCE_CONTINUE;
            if (!request.resizeStarted) {
                if (!this._resizeAllowed(request)) return GLib.SOURCE_REMOVE;
                const monitorIndex = window.get_monitor();
                const monitor = Main.layoutManager.monitors?.[monitorIndex];
                if (!Number.isInteger(monitorIndex) || monitorIndex < 0 || !monitor ||
                    !Number.isInteger(monitor.x) || !Number.isInteger(monitor.y) ||
                    !Number.isInteger(monitor.width) || monitor.width <= 0 ||
                    !Number.isInteger(monitor.height) || monitor.height <= 0) {
                    this._finishResize(request, 'Minecraft monitor geometry is unavailable');
                    return GLib.SOURCE_REMOVE;
                }
                request.x = monitor.x + Math.floor((monitor.width - request.width) / 2);
                request.y = monitor.y + Math.floor((monitor.height - request.height) / 2);
                if (!this._resizeAllowed(request)) return GLib.SOURCE_REMOVE;
                request.resizeStarted = true;
                window.move_resize_frame(true, request.x, request.y, request.width, request.height);
                return GLib.SOURCE_CONTINUE;
            }
            const frame = window.get_frame_rect();
            const content = window.get_client_content_rect();
            if (frame.x === request.x && frame.y === request.y &&
                frame.width === request.width && frame.height === request.height &&
                content.width === request.width && content.height === request.height) {
                this._finishResize(request, null, [content.width, content.height]);
                return GLib.SOURCE_REMOVE;
            }
        } catch (error) {
            this._finishResize(request, `Minecraft resize failed: ${error}`);
            return GLib.SOURCE_REMOVE;
        }
        return GLib.SOURCE_CONTINUE;
    }

    ResizeFocusedAsync([width, height], invocation) {
        const fail = message => invocation.return_dbus_error('org.mcsr.Toolkit.ResizeFailed', message);
        const window = minecraftWindow();
        if (!window) {
            fail('no focused Minecraft window');
            return;
        }
        if (!Number.isInteger(width) || !Number.isInteger(height) ||
            width < 100 || height < 100 || width > 16384 || height > 16384) {
            fail('invalid Minecraft window size');
            return;
        }
        if (this._resizeRequests.size > 0) {
            fail('a resize is already pending');
            return;
        }
        let request;
        try {
            request = {window, invocation, width, height, x: null, y: null,
                deadline: GLib.get_monotonic_time() + 2000000, timer: 0, process: null,
                done: false, prepared: false, statesRequested: false,
                preparationStarted: false, resizeStarted: false};
            this._resizeRequests.set(request, request);
            request.unmanagedSignal = window.connect?.('unmanaged', () =>
                this._finishResize(request, 'Minecraft window was unmanaged during resize'));
            request.timer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 25, () =>
                this._observeResize(request));
            this._observeResize(request);
        } catch (error) {
            if (request)
                this._finishResize(request, `Minecraft resize failed: ${error}`);
            else
                fail(`${error}`);
        }
    }
}
