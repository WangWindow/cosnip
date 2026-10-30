import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

export default class CosnipPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const settings = this.getSettings();
        window.add(this._generalPage(settings, window));
        window.add(this._annotationPage(settings));
        window.add(this._outputPage(settings, window));
        window.add(this._pinPage(settings));
        window.add(this._aboutPage());
    }

    _generalPage(settings, window) {
        const page = new Adw.PreferencesPage({
            title: 'General',
            icon_name: 'preferences-system-symbolic',
        });
        const group = new Adw.PreferencesGroup({title: 'Shortcuts'});
        page.add(group);

        group.add(this._shortcutRow(settings, 'snip-shortcut', 'Snip'));
        group.add(this._shortcutRow(settings, 'paste-shortcut', 'Paste as pin'));

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
            title: 'Annotations',
            icon_name: 'document-edit-symbolic',
        });
        const group = new Adw.PreferencesGroup({title: 'Defaults'});
        page.add(group);

        const color = new Adw.EntryRow({
            title: 'Color (#RRGGBB or #RRGGBBAA)',
            text: settings.get_string('annotation-color'),
        });
        color.connect('notify::text', () => settings.set_string('annotation-color', color.text));
        group.add(color);

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
        const note = new Adw.ActionRow({
            title: 'Native pins',
            subtitle: 'Pins are Shell actors: image-sized, always on top, and sharp on scaled displays.',
        });
        group.add(note);
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
        const row = new Adw.EntryRow({
            title: `${title} (comma-separated accelerator)`,
            text: settings.get_strv(key).join(', '),
        });
        row.connect('notify::text', () => {
            const values = row.text
                .split(',')
                .map(value => value.trim())
                .filter(value => value.length > 0);
            if (values.length === 0)
                values.push(key === 'paste-shortcut' ? 'F3' : 'F1');
            settings.set_strv(key, values);
        });
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
