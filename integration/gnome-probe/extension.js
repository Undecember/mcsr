import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const XML = `<node>
  <interface name="org.mcsr.Toolkit.Probe">
    <method name="GetFocus">
      <arg name="class" type="s" direction="out"/>
      <arg name="title" type="s" direction="out"/>
      <arg name="width" type="u" direction="out"/>
      <arg name="height" type="u" direction="out"/>
    </method>
    <method name="GetInputContext">
      <arg name="class" type="s" direction="out"/>
      <arg name="modal" type="b" direction="out"/>
    </method>
    <method name="ResizeFocused">
      <arg name="width" type="u" direction="in"/>
      <arg name="height" type="u" direction="in"/>
      <arg name="accepted" type="b" direction="out"/>
    </method>
  </interface>
</node>`;

function focusedWindow() {
    return global.display.focus_window;
}

function isMinecraft(window) {
    if (!window)
        return false;
    return `${window.get_wm_class() ?? ''} ${window.get_title() ?? ''}`
        .toLowerCase().includes('minecraft');
}

export default class ProbeExtension extends Extension {
    enable() {
        this._dbus = Gio.DBusExportedObject.wrapJSObject(XML, this);
        this._dbus.export(Gio.DBus.session, '/org/mcsr/Toolkit/Probe');
        const fifoPath = `${GLib.get_user_runtime_dir()}/keyd.fifo`;
        if (!GLib.file_test(fifoPath, GLib.FileTest.EXISTS))
            GLib.spawn_sync(null, ['mkfifo', fifoPath], null, GLib.SpawnFlags.SEARCH_PATH, null);
        this._fifo = Gio.File.new_for_path(fifoPath);
        this._cancel = new Gio.Cancellable();
        this._focusSignal = global.display.connect('notify::focus-window', () => this._sendFocus());
        this._modal = Main.modalCount > 0;
        Main.layoutManager.connectObject('system-modal-opened', () => this._sendFocus(), this);
        this._modalTimer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 100, () => {
            const modal = Main.modalCount > 0;
            if (modal !== this._modal) {
                this._modal = modal;
                this._sendFocus();
            }
            return GLib.SOURCE_CONTINUE;
        });
        this._openFifo();
    }

    disable() {
        global.display.disconnect(this._focusSignal);
        Main.layoutManager.disconnectObject(this);
        GLib.source_remove(this._modalTimer);
        this._cancel?.cancel();
        this._pipe?.close(null);
        this._pipe = null;
        this._dbus?.unexport();
        this._dbus = null;
    }

    _openFifo() {
        this._fifo.append_to_async(0, 0, this._cancel, (file, result) => {
            try {
                this._pipe = file.append_to_finish(result);
                this._sendFocus();
            } catch (error) {
                if (!this._cancel.is_cancelled())
                    log(`MCSR probe FIFO unavailable: ${error}`);
            }
        });
    }

    _sendFocus() {
        if (!this._pipe)
            return;
        const window = focusedWindow();
        const modal = Main.modalCount > 0;
        const cls = modal ? 'system-modal' : window?.get_wm_class() ?? 'root';
        const title = modal ? '' : window?.get_title() ?? '';
        try {
            this._pipe.write(`${cls}\t${title}\n`, null);
        } catch (error) {
            log(`MCSR probe FIFO write failed: ${error}`);
            this._pipe = null;
            this._openFifo();
        }
    }

    GetFocus() {
        const window = focusedWindow();
        if (!window)
            return ['', '', 0, 0];
        const rect = window.get_frame_rect();
        return [window.get_wm_class() ?? '', window.get_title() ?? '', rect.width, rect.height];
    }

    GetInputContext() {
        return [focusedWindow()?.get_wm_class() ?? '', Main.modalCount > 0];
    }

    ResizeFocused(width, height) {
        const window = focusedWindow();
        if (Main.modalCount > 0 || !isMinecraft(window) || width < 100 || height < 100)
            return false;
        const rect = window.get_frame_rect();
        window.move_resize_frame(true, rect.x, rect.y, width, height);
        return true;
    }
}
