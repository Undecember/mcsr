import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
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
        for (const [source, invocation] of this._resizeRequests ?? []) {
            GLib.source_remove(source);
            invocation.return_dbus_error('org.mcsr.Toolkit.Disabled', 'extension disabled');
        }
        this._resizeRequests?.clear();
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
        try {
            const rect = window.get_frame_rect();
            window.move_resize_frame(true, rect.x, rect.y, width, height);
        } catch (error) {
            fail(`${error}`);
            return;
        }
        let attempts = 0;
        const source = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 25, () => {
            if (minecraftWindow() !== window) {
                this._resizeRequests.delete(source);
                fail('Minecraft focus lost during resize');
                return GLib.SOURCE_REMOVE;
            }
            const applied = window.get_frame_rect();
            if ((applied.width === width && applied.height === height) || ++attempts >= 40) {
                this._resizeRequests.delete(source);
                invocation.return_value(new GLib.Variant('(uu)', [applied.width, applied.height]));
                return GLib.SOURCE_REMOVE;
            }
            return GLib.SOURCE_CONTINUE;
        });
        this._resizeRequests.set(source, invocation);
    }
}
