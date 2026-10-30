import Adw from 'gi://Adw';
import Gdk from 'gi://Gdk?version=4.0';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

function formatShortcut(value) {
    if (!value)
        return 'Disabled';
    const [ok, keyval, mods] = Gtk.accelerator_parse(value);
    if (!ok)
        return value;
    return Gtk.accelerator_get_label(keyval, mods);
}

function isModifierKey(keyval) {
    return [
        Gdk.KEY_Shift_L, Gdk.KEY_Shift_R,
        Gdk.KEY_Control_L, Gdk.KEY_Control_R,
        Gdk.KEY_Alt_L, Gdk.KEY_Alt_R,
        Gdk.KEY_Super_L, Gdk.KEY_Super_R,
        Gdk.KEY_Meta_L, Gdk.KEY_Meta_R,
        Gdk.KEY_Hyper_L, Gdk.KEY_Hyper_R,
    ].includes(keyval);
}

export default class CosnipPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const settings = this.getSettings();
        window.add(this._generalPage(settings));
        window.add(this._snipPage(settings));
        window.add(this._annotationPage(settings));
        window.add(this._controlsPage());
        window.add(this._outputPage(settings, window));
        window.add(this._pinPage(settings));
        window.add(this._aboutPage());
    }

    _generalPage(settings) {
        const page = new Adw.PreferencesPage({
            title: 'General',
            icon_name: 'preferences-system-symbolic',
        });
        const group = new Adw.PreferencesGroup({title: 'Shortcuts'});
        page.add(group);

        group.add(this._shortcutRow(settings, 'snip-shortcut', 'Snip'));
        group.add(this._shortcutRow(settings, 'paste-shortcut', 'Paste as pin'));

        return page;
    }

    _snipPage(settings) {
        const page = new Adw.PreferencesPage({
            title: 'Snip',
            icon_name: 'applets-screenshooter-symbolic',
        });
        const snip = new Adw.PreferencesGroup({title: 'Snipping'});
        page.add(snip);
        const tips = new Adw.SwitchRow({
            title: 'Show keyboard tips',
            subtitle: 'Display the shortcut panel while snipping',
        });
        settings.bind('show-tips', tips, 'active', Gio.SettingsBindFlags.DEFAULT);
        snip.add(tips);
        const magnifier = new Adw.SwitchRow({
            title: 'Show pixel magnifier',
        });
        settings.bind('magnifier', magnifier, 'active', Gio.SettingsBindFlags.DEFAULT);
        snip.add(magnifier);

        const detect = new Adw.SwitchRow({
            title: 'Auto detect windows',
            subtitle: 'Highlight the window under the pointer; press Tab to toggle',
        });
        settings.bind('auto-detect-windows', detect, 'active', Gio.SettingsBindFlags.DEFAULT);
        snip.add(detect);
        const hide1x1 = new Adw.SwitchRow({
            title: 'Ignore 1x1 windows',
        });
        settings.bind('hide-1x1', hide1x1, 'active', Gio.SettingsBindFlags.DEFAULT);
        snip.add(hide1x1);
        const captureCursor = new Adw.SwitchRow({
            title: 'Capture the pointer',
        });
        settings.bind('capture-cursor', captureCursor, 'active', Gio.SettingsBindFlags.DEFAULT);
        snip.add(captureCursor);

        snip.add(this._intRow(settings, 'border-width', 'Selection border width', 1, 12, ' px'));
        snip.add(this._colorRow(settings, 'border-color', 'Selection border color'));
        snip.add(this._colorRow(settings, 'mask-color', 'Mask color (#RRGGBBAA)'));
        const anchors = new Adw.SwitchRow({
            title: 'Show selection anchors',
        });
        settings.bind('show-anchors', anchors, 'active', Gio.SettingsBindFlags.DEFAULT);
        snip.add(anchors);
        snip.add(this._colorRow(settings, 'anchor-color', 'Anchor border color'));

        const magnifierGroup = new Adw.PreferencesGroup({title: 'Magnifier'});
        page.add(magnifierGroup);
        magnifierGroup.add(this._intRow(settings, 'magnifier-size', 'Grid size', 3, 21, ' px'));
        magnifierGroup.add(this._intRow(settings, 'magnifier-zoom', 'Pixel cell size', 2, 20, ' px'));
        const grid = new Adw.SwitchRow({title: 'Show pixel grid'});
        settings.bind('magnifier-grid', grid, 'active', Gio.SettingsBindFlags.DEFAULT);
        magnifierGroup.add(grid);
        const coords = new Adw.SwitchRow({title: 'Show coordinates and color'});
        settings.bind('magnifier-coords', coords, 'active', Gio.SettingsBindFlags.DEFAULT);
        magnifierGroup.add(coords);

        const actions = new Adw.PreferencesGroup({title: 'Actions'});
        page.add(actions);
        actions.add(this._actionRow(settings, 'enter-action', 'Enter'));
        actions.add(this._actionRow(settings, 'double-click-action', 'Double-click'));
        actions.add(this._actionRow(settings, 'middle-click-action', 'Middle-click'));
        snip.add(this._intRow(settings, 'snip-history-size', 'Recent snip areas', 1, 64, ''));
        const disableScroll = new Adw.SwitchRow({title: 'Disable mouse-wheel width changes'});
        settings.bind('disable-scroll', disableScroll, 'active', Gio.SettingsBindFlags.DEFAULT);
        snip.add(disableScroll);
        const crossLines = new Adw.SwitchRow({title: 'Show fullscreen cross lines'});
        settings.bind('show-cross-lines', crossLines, 'active', Gio.SettingsBindFlags.DEFAULT);
        snip.add(crossLines);

        const cursor = new Adw.ComboRow({
            title: 'Cursor',
            model: Gtk.StringList.new(['Auto', 'Always visible', 'Hidden']),
        });
        cursor.selected = Math.max(0, settings.get_enum('cursor-mode'));
        cursor.connect('notify::selected', () => {
            settings.set_enum('cursor-mode', cursor.selected);
        });
        snip.add(cursor);
        return page;
    }

    _annotationPage(settings) {
        const page = new Adw.PreferencesPage({
            title: 'Interface',
            icon_name: 'preferences-desktop-symbolic',
        });
        const group = new Adw.PreferencesGroup({title: 'Defaults'});
        page.add(group);

        group.add(this._colorRow(settings, 'annotation-color', 'Color (#RRGGBB or #RRGGBBAA)'));

        group.add(this._intRow(settings, 'annotation-width', 'Line width', 1, 32, ' px'));
        group.add(this._intRow(settings, 'font-size', 'Text size', 6, 96, ' px'));
        return page;
    }

    _outputPage(settings, window) {
        const page = new Adw.PreferencesPage({
            title: 'Output',
            icon_name: 'document-save-symbolic',
        });
        const group = new Adw.PreferencesGroup({title: 'Default action'});
        page.add(group);

        const output = new Adw.ComboRow({
            title: 'Action',
            model: Gtk.StringList.new(['Copy', 'Save', 'Pin']),
        });
        output.selected = Math.max(0, settings.get_enum('default-output'));
        output.connect('notify::selected', () => {
            settings.set_enum('default-output', output.selected);
        });
        group.add(output);

        const save = new Adw.PreferencesGroup({title: 'Save file'});
        page.add(save);
        const dir = new Adw.EntryRow({
            title: 'Directory (empty = Pictures)',
            text: settings.get_string('save-dir'),
        });
        dir.connect('notify::text', () => settings.set_string('save-dir', dir.text));
        save.add(dir);
        const browse = new Adw.ActionRow({title: 'Choose directory'});
        const button = new Gtk.Button({
            label: 'Browse…',
            valign: Gtk.Align.CENTER,
        });
        button.connect('clicked', () => {
            const dialog = new Gtk.FileDialog({title: 'Choose save directory'});
            dialog.select_folder(window, null, (dialog, result) => {
                try {
                    const file = dialog.select_folder_finish(result);
                    const path = file.get_path();
                    dir.text = path;
                    settings.set_string('save-dir', path);
                } catch (_error) {
                    // Cancelled.
                }
            });
        });
        browse.add_suffix(button);
        save.add(browse);

        const filename = new Adw.EntryRow({
            title: 'File name pattern',
            text: settings.get_string('filename'),
        });
        filename.connect('notify::text', () =>
            settings.set_string('filename', filename.text));
        save.add(filename);
        return page;
    }

    _pinPage(settings) {
        const page = new Adw.PreferencesPage({
            title: 'Pin',
            icon_name: 'view-pin-symbolic',
        });
        const group = new Adw.PreferencesGroup({title: 'Display'});
        page.add(group);
        const zoom = new Adw.SpinRow({
            title: 'Wheel zoom step',
            adjustment: new Gtk.Adjustment({
                lower: 0.0001,
                upper: 0.05,
                step_increment: 0.0001,
                page_increment: 0.001,
                value: settings.get_double('pin-zoom-step'),
            }),
            digits: 4,
        });
        zoom.connect('notify::value', () =>
            settings.set_double('pin-zoom-step', zoom.value));
        group.add(zoom);
        group.add(this._intRow(settings, 'pin-opacity', 'Default opacity', 10, 100, ' %'));
        group.add(this._intRow(settings, 'pin-border-width', 'Border width', 0, 8, ' px'));
        group.add(this._colorRow(settings, 'pin-border-color', 'Border color (#RRGGBBAA)'));
        const zoomAtPointer = new Adw.SwitchRow({
            title: 'Zoom around the pointer',
        });
        settings.bind('pin-zoom-at-pointer', zoomAtPointer, 'active', Gio.SettingsBindFlags.DEFAULT);
        group.add(zoomAtPointer);
        const note = new Adw.ActionRow({
            title: 'Native pins',
            subtitle: 'Pins are Shell actors: image-sized, always on top, and sharp on scaled displays.',
        });
        group.add(note);
        return page;
    }

    _controlsPage() {
        const page = new Adw.PreferencesPage({
            title: 'Controls',
            icon_name: 'input-keyboard-symbolic',
        });
        const keys = new Adw.PreferencesGroup({title: 'In-app shortcuts'});
        page.add(keys);
        for (const [key, action] of [
            ['R / C / L / A / B / H / M / T / N / G / E / V', 'Switch annotation tools'],
            ['Tab', 'Toggle window detection'],
            ['R / Shift+R', 'Reuse or cycle recent snip areas'],
            ['Enter', 'Configured Enter action'],
            ['Ctrl+C', 'Copy'],
            ['Ctrl+S', 'Save'],
            ['P', 'Pin'],
            ['Ctrl+Z / Ctrl+Shift+Z', 'Undo / redo'],
            ['Esc', 'Cancel tool, then close'],
        ]) {
            keys.add(new Adw.ActionRow({title: key, subtitle: action}));
        }
        const mouse = new Adw.PreferencesGroup({title: 'Mouse'});
        page.add(mouse);
        for (const [key, action] of [
            ['Drag', 'Create a selection; drag inside to move'],
            ['Drag a handle', 'Resize the selection'],
            ['Right-click', 'Finish the current tool or return to Select'],
            ['Scroll', 'Change tool width'],
        ]) {
            mouse.add(new Adw.ActionRow({title: key, subtitle: action}));
        }
        return page;
    }

    _aboutPage() {
        const page = new Adw.PreferencesPage({
            title: 'About',
            icon_name: 'help-about-symbolic',
        });
        const group = new Adw.PreferencesGroup({title: 'Cosnip'});
        page.add(group);
        group.add(new Adw.ActionRow({
            title: 'Native GNOME Shell extension',
            subtitle: 'Screenshot, annotate, copy, save, and pin without XWayland.',
        }));
        group.add(new Adw.ActionRow({
            title: 'Version',
            subtitle: '1',
        }));
        return page;
    }

    _shortcutRow(settings, key, title) {
        const row = new Adw.ActionRow({title});
        const button = new Gtk.Button({
            label: formatShortcut(settings.get_strv(key)[0]),
            valign: Gtk.Align.CENTER,
        });
        button.add_css_class('flat');
        row.add_suffix(button);
        row.set_activatable_widget(button);

        button.connect('clicked', () => {
            const controller = new Gtk.EventControllerKey();
            controller.set_propagation_phase(Gtk.PropagationPhase.CAPTURE);
            button.add_controller(controller);
            button.label = 'Press shortcut…';
            button.grab_focus();

            const finish = () => {
                button.remove_controller(controller);
                button.label = formatShortcut(settings.get_strv(key)[0]);
            };
            controller.connect('key-pressed', (_controller, keyval, _keycode, state) => {
                if (keyval === Gdk.KEY_Escape) {
                    finish();
                    return true;
                }
                if (keyval === Gdk.KEY_BackSpace || keyval === Gdk.KEY_Delete) {
                    settings.set_strv(key, []);
                    finish();
                    return true;
                }
                if (isModifierKey(keyval))
                    return true;
                const mods = state & (
                    Gdk.ModifierType.CONTROL_MASK |
                    Gdk.ModifierType.SHIFT_MASK |
                    Gdk.ModifierType.ALT_MASK |
                    Gdk.ModifierType.SUPER_MASK);
                if (!Gtk.accelerator_valid(keyval, mods))
                    return true;
                const accelerator = Gtk.accelerator_name(keyval, mods);
                if (!accelerator)
                    return true;
                settings.set_strv(key, [accelerator]);
                finish();
                return true;
            });
        });
        return row;
    }

    _actionRow(settings, key, title) {
        const row = new Adw.ComboRow({
            title,
            model: Gtk.StringList.new(['Copy', 'Save', 'Pin', 'Close']),
        });
        row.selected = Math.max(0, settings.get_enum(key));
        row.connect('notify::selected', () => settings.set_enum(key, row.selected));
        return row;
    }

    _colorRow(settings, key, title) {
        const row = new Adw.EntryRow({
            title,
            text: settings.get_string(key),
        });
        row.connect('notify::text', () => settings.set_string(key, row.text));
        return row;
    }

    _intRow(settings, key, title, lower, upper, suffix) {
        const row = new Adw.SpinRow({
            title,
            adjustment: new Gtk.Adjustment({
                lower,
                upper,
                step_increment: 1,
                page_increment: 5,
                value: settings.get_int(key),
            }),
        });
        row.connect('notify::value', () =>
            settings.set_int(key, Math.round(row.value)));
        return row;
    }
}
