import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Gtk from 'gi://Gtk?version=4.0';

const resource = GLib.get_system_data_dirs()
  .map(dir => GLib.build_filenamev([dir, 'gnome-shell',
    'org.gnome.Shell.Extensions.src.gresource']))
  .find(path => GLib.file_test(path, GLib.FileTest.EXISTS));
if (!resource)
  throw new Error('GNOME Shell extension preferences resources not found');
Gio.Resource.load(resource)._register();
const { default: Preferences } = await import('../cosnip@choco2233/prefs.js');
Gtk.init();
Adw.init();
const source = Gio.SettingsSchemaSource.new_from_directory(
  ARGV[0], Gio.SettingsSchemaSource.get_default(), false);
const schema = source.lookup('org.gnome.shell.extensions.cosnip', false);
const settings = new Gio.Settings({ settings_schema: schema });
const prefs = Object.create(Preferences.prototype);
const window = new Adw.PreferencesWindow();
const page = prefs._outputPage(settings, window);

function findByTitle(root, title) {
  if (typeof root.get_title === 'function' && root.get_title() === title)
    return root;
  for (let child = root.get_first_child(); child; child = child.get_next_sibling()) {
    const result = findByTitle(child, title);
    if (result)
      return result;
  }
  return null;
}

function assert(condition, message) {
  if (!condition)
    throw new Error(message);
}

const format = findByTitle(page, 'Image format');
const quality = findByTitle(page, 'Near-lossless quality');
const saveOnCopy = findByTitle(page, 'Save image when copying');
assert(format && !quality, 'PNG/JPEG format control is missing or obsolete quality remains');
assert(saveOnCopy, 'save-on-copy control is missing');
assert(format.selected === 0, 'PNG must be the default format');
format.selected = 1;
assert(settings.get_enum('save-format') === 1, 'JPEG selection did not persist');
settings.set_enum('save-format', 0);
assert(format.selected === 0, 'external format change was not reflected');
saveOnCopy.active = true;
assert(settings.get_boolean('save-on-copy'), 'save-on-copy change did not persist');
window.destroy();
print('Output preferences: PNG/JPEG format control verified');
