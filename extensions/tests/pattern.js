import Gtk from 'gi://Gtk?version=4.0';
import GLib from 'gi://GLib';

Gtk.init();
const window = new Gtk.Window({
  title: 'Cosnip pixel test', default_width: 800, default_height: 550,
  decorated: false,
});
const canvas = new Gtk.DrawingArea({ hexpand: true, vexpand: true });
canvas.set_draw_func((_area, cr, width, height) => {
  cr.setSourceRGB(1, 1, 1);
  cr.paint();
  cr.setSourceRGB(0, 0, 0);
  for (let y = 0; y < height; y += 2) {
    for (let x = (y / 2) % 2; x < width; x += 2)
      cr.rectangle(x, y, 1, 1);
  }
  cr.fill();
});
window.set_child(canvas);
window.present();
new GLib.MainLoop(null, false).run();
