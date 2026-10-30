import Adw from 'gi://Adw';
import Gdk from 'gi://Gdk?version=4.0';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import { ExtensionPreferences } from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

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
    window.add(this._outputPage(settings, window));
    window.add(this._pinPage(settings));
    window.add(this._aboutPage());
  }

  _generalPage(settings) {
    const page = new Adw.PreferencesPage({
      title: 'General',
      icon_name: 'preferences-system-symbolic',
    });
    const group = new Adw.PreferencesGroup({ title: 'Shortcuts' });
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
    const group = new Adw.PreferencesGroup({ title: 'Snipping' });
    page.add(group);

    const tips = new Adw.SwitchRow({
      title: 'Show keyboard tips',
      subtitle: 'Display the shortcut panel while snipping',
    });
    settings.bind('show-tips', tips, 'active', Gio.SettingsBindFlags.DEFAULT);
    group.add(tips);

    const magnifier = new Adw.SwitchRow({
      title: 'Show pixel magnifier',
    });
    settings.bind('magnifier', magnifier, 'active', Gio.SettingsBindFlags.DEFAULT);
    group.add(magnifier);

    const detect = new Adw.SwitchRow({
      title: 'Auto detect windows',
      subtitle: 'Highlight the window under the pointer; press Tab to toggle',
    });
    settings.bind('auto-detect-windows', detect, 'active', Gio.SettingsBindFlags.DEFAULT);
    group.add(detect);
    return page;
  }

  _outputPage(settings, window) {
    const page = new Adw.PreferencesPage({
      title: 'Output',
      icon_name: 'document-save-symbolic',
    });
    const group = new Adw.PreferencesGroup({ title: 'Default action' });
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

    const save = new Adw.PreferencesGroup({ title: 'Save file' });
    page.add(save);
    const dir = new Adw.EntryRow({
      title: 'Directory (empty = Pictures)',
      text: settings.get_string('save-dir'),
    });
    dir.connect('notify::text', () => settings.set_string('save-dir', dir.text));
    save.add(dir);
    const browse = new Adw.ActionRow({ title: 'Choose directory' });
    const button = new Gtk.Button({
      label: 'Browse…',
      valign: Gtk.Align.CENTER,
    });
    button.connect('clicked', () => {
      const dialog = new Gtk.FileDialog({ title: 'Choose save directory' });
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
    const group = new Adw.PreferencesGroup({ title: 'Display' });
    page.add(group);
    group.add(this._intRow(
      settings, 'pin-opacity', 'Default opacity', 10, 100, ' %'));
    group.add(new Adw.ActionRow({
      title: 'Zoom and close',
      subtitle: 'Scroll over a pin to zoom; right-click to close it',
    }));
    return page;
  }

  _aboutPage() {
    const page = new Adw.PreferencesPage({
      title: 'About',
      icon_name: 'help-about-symbolic',
    });
    const group = new Adw.PreferencesGroup({ title: 'Cosnip' });
    page.add(group);
    group.add(new Adw.ActionRow({
      title: 'Native GNOME Shell extension',
      subtitle: 'Screenshot, annotate, copy, save, and pin without X11.',
    }));
    group.add(new Adw.ActionRow({
      title: 'Author',
      subtitle: 'choco2233',
    }));
    group.add(new Adw.ActionRow({
      title: 'Version',
      subtitle: '2',
    }));
    return page;
  }

  _shortcutRow(settings, key, title) {
    const row = new Adw.ActionRow({ title });
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
