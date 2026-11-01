import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import cairo from 'cairo';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {
  Tool,
  annotationIsValid,
  createAnnotation,
  drawAnnotations,
  updateAnnotation,
} from './annotations.js';
import { PALETTE, WIDTHS, Toolbar } from './toolbar.js';
import {
  copySurfaceToClipboard,
  defaultSaveDir,
  expandFilename,
  paintPixbuf,
  pixbufPixel,
  saveSurface,
  uniquePath,
} from './util.js';

const HANDLE_RADIUS = 8;
const BORDER_WIDTH = 2;
const BORDER_COLOR = [59, 130, 246, 255];
const MASK_COLOR = [0, 0, 0, 100];
const ANCHOR_COLOR = [59, 130, 246, 255];
const ANNOTATION_COLOR = [255, 64, 64, 255];
const ANNOTATION_WIDTH = 3;
const FONT_SIZE = 20;
const MAGNIFIER_SIZE = 9;
const MAGNIFIER_ZOOM = 8;

export class NativeOverlay {
  constructor(options) {
    this._content = options.content;
    this._scale = options.scale || 1;
    this._cursorContent = options.cursorContent ?? null;
    this._cursorPoint = options.cursorPoint ?? null;
    this._cursorScale = options.cursorScale ?? 1;
    this._showCursor = false;
    this._settings = options.settings;
    this._onClose = options.onClose;
    this._onPin = options.onPin;
    this._selectionPixbuf = null;
    this._selectionPixbufRect = null;
    this._selectionCapturePromise = null;
    this._captureGeneration = 0;
    this._annotations = [];
    this._undo = [];
    this._redo = [];
    this._draft = null;
    this._selection = null;
    this._tool = Tool.SELECT;
    this._style = {
      color: ANNOTATION_COLOR,
      width: ANNOTATION_WIDTH,
      fontSize: FONT_SIZE,
    };
    this._drag = null;
    this._textEntry = null;
    this._closed = false;
    this._autoDetect = this._settings.get_boolean('auto-detect-windows');
    this._borderWidth = BORDER_WIDTH;
    this._borderColor = BORDER_COLOR;
    this._maskColor = MASK_COLOR;
    this._showAnchors = true;
    this._anchorColor = ANCHOR_COLOR;
    this._magnifier = this._settings.get_boolean('magnifier');
    this._magnifierSize = MAGNIFIER_SIZE;
    this._magnifierZoom = MAGNIFIER_ZOOM;
    this._magnifierGrid = true;
    this._magnifierCoords = true;
    this._pointerPos = null;
    this._lastClick = null;
    this._hoverWindow = null;
    this._windowList = [];
    this._root = null;
    this._maskWidgets = [];
    this._frameBox = null;
    this._innerFrame = null;
    this._handleWidgets = [];
    this._annotationsArea = null;
    this._magnifierArea = null;
    this._lastMagnifierUpdate = 0;
    this._toolbar = null;
    this._sizeBadge = null;
    this._tips = null;
    this._tooltip = null;
    this._popup = null;
  }

  async open() {
    try {
      this._buildUI();
    } catch (error) {
      this._close();
      throw error;
    }
  }

  get actor() {
    return this._root;
  }

  get toolbar() {
    return this._toolbar;
  }

  debugState() {
    const [tx, ty] = this._toolbar?.visible
      ? this._toolbar.get_transformed_position() : [0, 0];
    const [tw, th] = this._toolbar?.visible
      ? this._toolbar.get_transformed_size() : [0, 0];
    const buttons = [];
    if (this._toolbar) {
      for (const child of this._toolbar.get_children()) {
        const [x, y] = child.get_transformed_position();
        const [w, h] = child.get_transformed_size();
        buttons.push({
          x, y, w, h, visible: child.visible,
          name: child.accessible_name,
        });
      }
    }
    const popup = this._popup ? (() => {
      const [x, y] = this._popup.get_transformed_position();
      const [w, h] = this._popup.get_transformed_size();
      const items = this._popup.get_children().map(child => {
        const [ix, iy] = child.get_transformed_position();
        const [iw, ih] = child.get_transformed_size();
        return { x: ix, y: iy, w: iw, h: ih };
      });
      return { x, y, w, h, items };
    })() : null;
    return JSON.stringify({
      stage: { w: global.stage.width, h: global.stage.height },
      pointer: global.get_pointer(),
      selection: this._selection,
      tool: this._tool, source: this._selectionPixbuf ? 'yes' : 'no',
      cursor: this._showCursor,
      cursorContent: Boolean(this._cursorContent),
      cursorPoint: this._cursorPoint,
      cursorType: this._root?.cursor_type,
      textEntry: Boolean(this._textEntry),
      tooltip: this._tooltip?.visible ? this._tooltip.get_text() : null,
      color: this._style.color,
      width: this._style.width,
      toolbar: { x: tx, y: ty, w: tw, h: th, visible: this._toolbar?.visible },
      buttons,
      popup,
    });
  }

  get selection() {
    return this._selection ? { ...this._selection } : null;
  }

  get tool() {
    return this._tool;
  }

  get hoverWindow() {
    return this._hoverWindow ? { ...this._hoverWindow } : null;
  }

  testMotion(x, y) {
    this._onMotion({
      get_coords: () => [x, y],
      get_state: () => 0,
      get_source: () => this._root,
    });
  }

  testRelease() {
    this._onRelease({
      get_button: () => 1,
      get_coords: () => [0, 0],
      get_state: () => 0,
      get_source: () => this._root,
    });
  }

  testPress(x, y) {
    this._onPress({
      get_coords: () => [x, y],
      get_button: () => 1,
      get_state: () => 0,
      get_source: () => this._root,
    });
  }

  setSelection(selection) {
    this._selection = clampSelection(selection);
    this._refreshSelectionPixbuf();
    this._layoutChrome();
    this._repaint();
  }

  addRect(x, y, w, h) {
    this._addAnnotation({
      kind: Tool.RECT,
      x, y, w, h,
      color: this._style.color,
      width: this._style.width,
    });
    this._repaint();
  }

  addMosaic(x1, y1, x2, y2) {
    this._addAnnotation({
      kind: Tool.MOSAIC,
      points: [{ x: x1, y: y1 }, { x: x2, y: y2 }],
      color: this._style.color,
      width: this._style.width,
    });
    this._repaint();
  }

  addText(text, x, y) {
    this._addAnnotation({
      kind: Tool.TEXT,
      x, y, text,
      size: this._style.fontSize,
      color: this._style.color,
      width: this._style.width,
    });
    this._repaint();
  }

  addEraser(x1, y1, x2, y2) {
    this._addAnnotation({
      kind: Tool.ERASER,
      points: [{ x: x1, y: y1 }, { x: x2, y: y2 }],
      color: this._style.color,
      width: this._style.width,
    });
    this._repaint();
  }

  pinSelection() {
    this._pin();
  }

  async saveTo(path) {
    await this._ensureSelectionPixbuf();
    const surface = this._renderSelection();
    if (!surface)
      return false;
    saveSurface(surface, path);
    return true;
  }

  _refreshSelectionPixbuf() {
    if (!this._selection) {
      this._selectionPixbuf = null;
      this._selectionPixbufRect = null;
      this._selectionCapturePromise = null;
      return;
    }
    this._selectionCapturePromise = this._captureSelectionPixbuf();
  }

  async _captureSelectionPixbuf() {
    const selection = this._selection;
    if (!selection)
      return;
    const generation = ++this._captureGeneration;
    const sel = {
      x: Math.round(selection.x),
      y: Math.round(selection.y),
      w: Math.max(1, Math.round(selection.w)),
      h: Math.max(1, Math.round(selection.h)),
    };
    const texture = this._content.get_texture();
    const stream = Gio.MemoryOutputStream.new_resizable();
    const cursorTexture = this._showCursor && this._cursorContent
      ? this._cursorContent.get_texture()
      : null;
    const cursorX = this._cursorPoint ? this._cursorPoint.x * this._scale : 0;
    const cursorY = this._cursorPoint ? this._cursorPoint.y * this._scale : 0;
    try {
      const pixbuf = await Shell.Screenshot.composite_to_stream(
        texture,
        Math.round(sel.x * this._scale),
        Math.round(sel.y * this._scale),
        Math.round(sel.w * this._scale),
        Math.round(sel.h * this._scale),
        this._scale,
        cursorTexture, cursorX, cursorY, this._cursorScale,
        stream);
      stream.close(null);
      if (this._closed || generation !== this._captureGeneration)
        return;
      this._selectionPixbuf = pixbuf;
      this._selectionPixbufRect = sel;
      this._selectionCapturePromise = null;
      this._repaint();
      log(`cosnip: selection pixels ${pixbuf.get_width()}x${pixbuf.get_height()}`);
    } catch (error) {
      stream.close(null);
      this._selectionCapturePromise = null;
      logError(error, 'cosnip: cannot capture selection pixels');
    }
  }

  async _ensureSelectionPixbuf() {
    if (!this._selection)
      return false;
    const sel = {
      x: Math.round(this._selection.x),
      y: Math.round(this._selection.y),
      w: Math.max(1, Math.round(this._selection.w)),
      h: Math.max(1, Math.round(this._selection.h)),
    };
    if (this._selectionPixbuf &&
      sameRect(this._selectionPixbufRect, sel))
      return true;
    if (this._selectionCapturePromise)
      await this._selectionCapturePromise;
    if (!this._selectionPixbuf ||
      !sameRect(this._selectionPixbufRect, sel)) {
      await this._captureSelectionPixbuf();
    }
    return Boolean(this._selectionPixbuf);
  }

  _buildUI() {
    const stageWidth = global.stage.width;
    const stageHeight = global.stage.height;
    this._root = new St.Widget({
      name: 'cosnip-overlay',
      reactive: true,
      can_focus: true,
      cursor_type: Clutter.CursorType.CROSSHAIR,
      layout_manager: new Clutter.BinLayout(),
      width: stageWidth,
      height: stageHeight,
    });

    this._background = new St.Widget({
      content: this._content,
      width: stageWidth,
      height: stageHeight,
    });
    this._root.add_child(this._background);

    const maskStyle = `background-color: rgba(${this._maskColor[0]}, ` +
      `${this._maskColor[1]}, ${this._maskColor[2]}, ` +
      `${this._maskColor[3] / 255});`;
    for (let i = 0; i < 4; i++) {
      const mask = new St.Widget({
        reactive: false,
        style: maskStyle,
      });
      this._maskWidgets.push(mask);
      this._root.add_child(mask);
    }

    this._annotationsArea = new St.DrawingArea({ reactive: false });
    this._annotationsArea.connect('repaint',
      area => this._paintAnnotations(area));
    this._root.add_child(this._annotationsArea);

    const frameColor = `rgba(${this._borderColor[0]}, ` +
      `${this._borderColor[1]}, ${this._borderColor[2]}, ` +
      `${this._borderColor[3] / 255})`;
    this._frameBox = new St.Widget({
      reactive: false,
      style: `border: ${this._borderWidth}px solid ${frameColor};`,
    });
    this._innerFrame = new St.Widget({
      reactive: false,
      style: 'border: 1px solid rgba(255, 255, 255, 0.28);',
    });
    this._root.add_child(this._frameBox);
    this._root.add_child(this._innerFrame);
    const anchorColor = `rgba(${this._anchorColor[0]}, ` +
      `${this._anchorColor[1]}, ${this._anchorColor[2]}, ` +
      `${this._anchorColor[3] / 255})`;
    for (let i = 0; i < 8; i++) {
      const handle = new St.Widget({
        reactive: false,
        width: 9,
        height: 9,
        style: 'background-color: white; ' +
          `border: 1.2px solid ${anchorColor}; ` +
          'border-radius: 5px;',
      });
      this._handleWidgets.push(handle);
      this._root.add_child(handle);
    }

    this._magnifierArea = new St.DrawingArea({
      reactive: false,
      visible: false,
    });
    this._magnifierArea.connect('repaint',
      area => this._paintMagnifier(area));
    this._root.add_child(this._magnifierArea);

    this._toolbar = new Toolbar({
      onTool: tool => this._setTool(tool),
      onCancel: () => this._close(),
      onPin: () => this._pin(),
      onSave: () => this._save(),
      onCopy: () => this._copy(),
      onColor: color => {
        this._style.color = color;
      },
      onWidth: width => {
        this._style.width = width;
      },
      onColorPicker: anchor => this._showColorPicker(anchor),
      onWidthPicker: anchor => this._showWidthPicker(anchor),
      onCursor: visible => {
        this._showCursor = visible;
        this._refreshSelectionPixbuf();
      },
      onHover: (label, button) => this._showTooltip(label, button),
    });
    this._toolbar.setStyle(this._style.color, this._style.width);
    this._toolbar.visible = false;
    this._root.add_child(this._toolbar);

    this._sizeBadge = new St.Label({
      style_class: 'cosnip-size-badge',
      visible: false,
    });
    this._root.add_child(this._sizeBadge);

    this._tips = new St.Label({
      style_class: 'cosnip-tips',
      visible: this._settings.get_boolean('show-tips'),
    });
    this._tips.clutter_text.set_markup(
      '<b>Drag</b> select   <b>Shift</b> square   <b>Enter</b> copy\n' +
      '<b>Ctrl+S</b> save   <b>P</b> pin   <b>Tab</b> window detect   <b>Esc</b> cancel');
    this._root.add_child(this._tips);

    this._tooltip = new St.Label({
      style_class: 'cosnip-tooltip',
      visible: false,
    });
    this._root.add_child(this._tooltip);

    this._root.connect('button-press-event', (_actor, event) => this._onPress(event));
    this._root.connect('button-release-event', (_actor, event) => this._onRelease(event));
    this._root.connect('motion-event', (_actor, event) => this._onMotion(event));
    this._root.connect('key-press-event', (_actor, event) => this._onKey(event));
    this._root.connect('scroll-event', (_actor, event) => this._onScroll(event));
    this._root.connect('destroy', () => {
      try {
        if (this._panelBox)
          this._panelBox.visible = this._panelVisible ?? true;
      } catch (_error) {
        // The panel may already be gone while the Shell is shutting down.
      }
    });

    this._windowList = this._collectWindows();
    Main.layoutManager.screenshotUIGroup.add_child(this._root);
    this._panelBox = Main.layoutManager.panelBox;
    this._panelVisible = this._panelBox.visible;
    this._panelBox.hide();
    this._root.show();
    this._root.opacity = 0;
    this._root.ease({
      opacity: 255,
      duration: 110,
      mode: Clutter.AnimationMode.EASE_OUT_QUAD,
    });
    this._grab = Main.pushModal(this._root, {
      actionMode: Shell.ActionMode.POPUP,
    });
    this._layoutChrome();
    this._repaint();
  }

  _layoutMask() {
    const width = global.stage.width;
    const height = global.stage.height;
    const rect = this._selection || this._hoverWindow;
    const masks = this._maskWidgets;
    if (!masks.length)
      return;
    if (!rect) {
      masks[0].set_position(0, 0);
      masks[0].set_size(width, height);
      masks[0].show();
      for (let i = 1; i < masks.length; i++)
        masks[i].hide();
      return;
    }
    const x = Math.max(0, Math.round(rect.x));
    const y = Math.max(0, Math.round(rect.y));
    const w = Math.max(0, Math.round(rect.w));
    const h = Math.max(0, Math.round(rect.h));
    const parts = [
      [0, 0, width, y],
      [0, y + h, width, Math.max(0, height - y - h)],
      [0, y, x, h],
      [x + w, y, Math.max(0, width - x - w), h],
    ];
    for (let i = 0; i < masks.length; i++) {
      const [mx, my, mw, mh] = parts[i];
      if (mw <= 0 || mh <= 0) {
        masks[i].hide();
        continue;
      }
      masks[i].set_position(mx, my);
      masks[i].set_size(mw, mh);
      masks[i].show();
    }
  }

  _layoutVisuals() {
    this._layoutMask();
    const rect = this._selection || this._hoverWindow;
    if (!rect) {
      this._frameBox.hide();
      this._innerFrame.hide();
      for (const handle of this._handleWidgets)
        handle.hide();
      this._annotationsArea.hide();
      this._updateMagnifier();
      return;
    }
    const x = Math.round(rect.x);
    const y = Math.round(rect.y);
    const w = Math.max(1, Math.round(rect.w));
    const h = Math.max(1, Math.round(rect.h));
    this._frameBox.set_position(x, y);
    this._frameBox.set_size(w, h);
    this._frameBox.show();
    if (w > 5 && h > 5) {
      this._innerFrame.set_position(x + 2, y + 2);
      this._innerFrame.set_size(w - 4, h - 4);
      this._innerFrame.show();
    } else {
      this._innerFrame.hide();
    }
    const showHandles = Boolean(this._selection && this._showAnchors);
    if (showHandles) {
      const points = handlePoints({ x, y, w, h });
      for (let i = 0; i < this._handleWidgets.length; i++) {
        const handle = this._handleWidgets[i];
        handle.set_position(
          Math.round(points[i][0]) - 4,
          Math.round(points[i][1]) - 4);
        handle.show();
      }
    } else {
      for (const handle of this._handleWidgets)
        handle.hide();
    }
    const dragging = this._drag &&
      (this._drag.mode === 'new' || this._drag.mode === 'move' ||
        this._drag.mode === 'resize');
    if (this._selection && !dragging) {
      this._annotationsArea.set_position(x, y);
      this._annotationsArea.set_size(w, h);
      this._annotationsArea.show();
    } else {
      this._annotationsArea.hide();
    }
    this._updateMagnifier();
  }

  _updateMagnifier() {
    if (!this._magnifierArea)
      return;
    if (!this._magnifier || !this._pointerPos ||
      !this._selection || !this._selectionPixbuf ||
      (this._drag && this._drag.mode !== 'new')) {
      this._magnifierArea.hide();
      return;
    }
    const now = GLib.get_monotonic_time() / 1000;
    if (this._magnifierArea.visible &&
      now - this._lastMagnifierUpdate < 33)
      return;
    this._lastMagnifierUpdate = now;
    const size = Math.max(3, Math.min(21, this._magnifierSize | 1));
    const cell = Math.max(2, this._magnifierZoom);
    const box = size * cell;
    const width = this._magnifierCoords
      ? Math.max(box + 8, 150) : box + 8;
    const height = box + 8 + (this._magnifierCoords ? 22 : 0);
    const stageWidth = global.stage.width;
    const stageHeight = global.stage.height;
    let x = this._pointerPos.x + 24;
    if (x + width + 8 > stageWidth)
      x = this._pointerPos.x - width - 24;
    x = Math.max(8, Math.min(stageWidth - width - 8, x));
    let y = this._pointerPos.y + 24;
    if (y + height + 8 > stageHeight)
      y = this._pointerPos.y - height - 24;
    y = Math.max(8, Math.min(stageHeight - height - 8, y));

    if (this._toolbar?.visible) {
      const [tx, ty] = this._toolbar.get_transformed_position();
      const [tw, th] = this._toolbar.get_transformed_size();
      const overlapsToolbar = x < tx + tw && x + width > tx &&
        y < ty + th + 8 && y + height > ty;
      if (overlapsToolbar) {
        y = this._pointerPos.y - height - 24;
        y = Math.max(8, Math.min(stageHeight - height - 8, y));
      }
    }

    const nextX = Math.round(x);
    const nextY = Math.round(y);
    const nextWidth = Math.round(width);
    const nextHeight = Math.round(height);
    if (this._magnifierArea.width !== nextWidth ||
      this._magnifierArea.height !== nextHeight)
      this._magnifierArea.set_size(nextWidth, nextHeight);
    if (this._magnifierArea.x !== nextX ||
      this._magnifierArea.y !== nextY)
      this._magnifierArea.set_position(nextX, nextY);
    this._magnifierArea.show();
    this._magnifierArea.queue_repaint();
  }

  _paintMagnifier(area) {
    if (!this._magnifier || !this._pointerPos ||
      !this._selectionPixbuf)
      return;
    const cr = area.get_context();
    const size = Math.max(3, Math.min(21, this._magnifierSize | 1));
    const radius = (size - 1) / 2;
    const cell = Math.max(2, this._magnifierZoom);
    const box = size * cell;
    const origin = this._selectionPixbufRect ?? { x: 0, y: 0 };
    const x = 4;
    const y = 4;

    cr.setSourceRGBA(0, 0, 0, 0.82);
    cr.rectangle(0, 0, area.width, area.height);
    cr.fill();
    for (let gy = 0; gy < size; gy++) {
      for (let gx = 0; gx < size; gx++) {
        const sx = Math.round(
          (this._pointerPos.x + gx - radius - origin.x) * this._scale);
        const sy = Math.round(
          (this._pointerPos.y + gy - radius - origin.y) * this._scale);
        const pixel = pixbufPixel(this._selectionPixbuf, sx, sy);
        cr.setSourceRGBA(
          pixel[0] / 255, pixel[1] / 255, pixel[2] / 255, 1);
        cr.rectangle(x + gx * cell, y + gy * cell, cell, cell);
        cr.fill();
      }
    }
    if (this._magnifierGrid) {
      cr.setSourceRGBA(0, 0, 0, 0.28);
      cr.setLineWidth(1);
      for (let i = 0; i <= size; i++) {
        cr.moveTo(x + i * cell, y);
        cr.lineTo(x + i * cell, y + box);
        cr.moveTo(x, y + i * cell);
        cr.lineTo(x + box, y + i * cell);
      }
      cr.stroke();
    }
    cr.setSourceRGBA(1, 1, 1, 0.9);
    cr.setLineWidth(1);
    cr.rectangle(x - 1, y - 1, box + 2, box + 2);
    cr.stroke();

    if (this._magnifierCoords) {
      const px = Math.round(this._pointerPos.x);
      const py = Math.round(this._pointerPos.y);
      const pixel = pixbufPixel(
        this._selectionPixbuf,
        Math.round((px - origin.x) * this._scale),
        Math.round((py - origin.y) * this._scale));
      const text = `${px}, ${py}  #${pixel[0].toString(16).padStart(2, '0')}` +
        `${pixel[1].toString(16).padStart(2, '0')}` +
        `${pixel[2].toString(16).padStart(2, '0')}`;
      cr.setSourceRGBA(0, 0, 0, 0.82);
      cr.rectangle(x - 4, y + box + 6, box + 8, 16);
      cr.fill();
      cr.setSourceRGBA(1, 1, 1, 0.92);
      cr.selectFontFace('monospace', cairo.FontSlant.NORMAL,
        cairo.FontWeight.NORMAL);
      cr.setFontSize(10);
      cr.moveTo(x, y + box + 17);
      cr.showText(text);
    }
  }

  _paintAnnotations(area) {
    if (!this._selection)
      return;
    const cr = area.get_context();
    const x = Math.round(this._selection.x);
    const y = Math.round(this._selection.y);
    cr.save();
    cr.rectangle(0, 0, area.width, area.height);
    cr.clip();
    cr.translate(-x, -y);
    drawAnnotations(cr, this._annotations, {
      sourcePixbuf: this._selectionPixbuf,
      sourceOrigin: this._selectionPixbufRect ?? this._selection,
      deviceScale: this._scale,
    });
    if (this._draft) {
      drawAnnotations(cr, [this._draft], {
        sourcePixbuf: this._selectionPixbuf,
        sourceOrigin: this._selectionPixbufRect ?? this._selection,
        deviceScale: this._scale,
      });
    }
    cr.restore();
  }

  _repaint() {
    this._annotationsArea?.queue_repaint();
    this._updateMagnifier();
  }

  _showColorPicker(anchor) {
    const items = PALETTE.map(color => ({
      value: color,
      actor: new St.Widget({
        width: 18,
        height: 18,
        style: 'border-radius: 3px; border: 1px solid rgba(0, 0, 0, 0.35); ' +
          `background-color: rgba(${color[0]}, ${color[1]}, ${color[2]}, ` +
          `${color[3] / 255});`,
      }),
    }));
    this._showPopup(anchor, items, color => {
      this._style.color = color;
      this._toolbar.setStyle(color, this._style.width);
    });
  }

  _showWidthPicker(anchor) {
    const items = WIDTHS.map(width => {
      const area = new St.DrawingArea({ width: 34, height: 18 });
      area.connect('repaint', widget => {
        const cr = widget.get_context();
        cr.setSourceRGBA(0.12, 0.12, 0.12, 1);
        cr.setLineWidth(Math.max(1, width));
        cr.moveTo(3, 9);
        cr.lineTo(31, 9);
        cr.stroke();
      });
      return { value: width, actor: area };
    });
    this._showPopup(anchor, items, width => {
      this._style.width = width;
      this._toolbar.setStyle(this._style.color, width);
    });
  }

  _showPopup(anchor, items, onSelect) {
    this._tooltip?.hide();
    this._closePopup();
    const popup = new St.BoxLayout({
      style_class: 'cosnip-popup',
      orientation: Clutter.Orientation.HORIZONTAL,
      reactive: true,
    });
    for (const item of items) {
      const button = new St.Button({
        style_class: 'cosnip-popup-item',
        reactive: true,
        can_focus: false,
        track_hover: true,
      });
      button.set_child(item.actor);
      button.connect('clicked', () => {
        onSelect(item.value);
        this._closePopup();
      });
      popup.add_child(button);
    }
    this._popup = popup;
    this._root.add_child(popup);
    const [ax, ay] = anchor.get_transformed_position();
    const [aw, ah] = anchor.get_transformed_size();
    const [, pw] = popup.get_preferred_width(-1);
    const [, ph] = popup.get_preferred_height(pw);
    let x = Math.round(ax + aw / 2 - pw / 2);
    x = Math.max(8, Math.min(global.stage.width - pw - 8, x));
    let y = Math.round(ay - ph - 6);
    if (y < 8)
      y = Math.round(ay + ah + 6);
    popup.set_position(x, y);
    popup.show();
  }

  _closePopup() {
    if (!this._popup)
      return;
    this._popup.destroy();
    this._popup = null;
  }

  _popupClickAt(point) {
    if (!this._popup?.visible)
      return false;
    for (const child of this._popup.get_children()) {
      if (!child.visible)
        continue;
      const [x, y] = child.get_transformed_position();
      const [w, h] = child.get_transformed_size();
      if (point.x >= x && point.x < x + w &&
        point.y >= y && point.y < y + h) {
        child.emit('clicked', child);
        return true;
      }
    }
    return false;
  }

  _layoutChrome() {
    this._layoutVisuals();
    if (!this._selection) {
      this._toolbar.visible = false;
      this._tooltip?.hide();
      this._sizeBadge.visible = false;
      this._tips.visible = this._settings.get_boolean('show-tips');
      this._tips.set_position(12, global.stage.height - 58);
      return;
    }
    const { x, y, w, h } = this._selection;
    const [minWidth, naturalWidth] = this._toolbar.get_preferred_width(-1);
    const [minHeight, naturalHeight] = this._toolbar.get_preferred_height(naturalWidth);
    const toolbarWidth = Math.max(minWidth, naturalWidth);
    const toolbarHeight = Math.max(minHeight, naturalHeight);
    let tx = Math.round(x + w / 2 - toolbarWidth / 2);
    tx = Math.max(8, Math.min(global.stage.width - toolbarWidth - 8, tx));
    let ty = Math.round(y + h + 8);
    if (ty + toolbarHeight > global.stage.height - 8)
      ty = Math.round(y - toolbarHeight - 8);
    ty = Math.max(8, ty);
    this._toolbar.set_position(tx, ty);
    this._toolbar.visible = !this._drag ||
      this._drag.mode === 'annotation';

    this._sizeBadge.set_text(`${Math.round(w)} x ${Math.round(h)} px`);
    this._sizeBadge.set_position(
      Math.round(x), Math.round(Math.max(4, y - 24)));
    this._sizeBadge.visible = true;

    const showTips = this._settings.get_boolean('show-tips');
    const tipsHeight = this._tips.get_preferred_height(-1)[1] || 48;
    const tipsTop = global.stage.height - tipsHeight - 12;
    const tipsBottom = tipsTop + tipsHeight;
    const tipsWidth = this._tips.get_preferred_width(-1)[1] || 250;
    const overlap = tx < 12 + tipsWidth && tx + toolbarWidth > 12 &&
      ty < tipsBottom && ty + toolbarHeight > tipsTop;
    this._tips.visible = showTips && !overlap;
    this._tips.set_position(12, Math.round(tipsTop));
  }

  _setTool(tool) {
    if (this._textEntry)
      this._commitText();
    this._tool = tool;
    this._toolbar.syncShape(tool);
    this._toolbar.setTool(tool);
  }

  _collectWindows() {
    const active = global.workspace_manager.get_active_workspace();
    const list = [];
    for (const actor of global.get_window_actors()) {
      const win = actor.meta_window;
      if (!win)
        continue;
      try {
        if (win.minimized || win.skip_taskbar)
          continue;
        const type = win.get_window_type();
        if (type !== Meta.WindowType.NORMAL &&
          type !== Meta.WindowType.DIALOG &&
          type !== Meta.WindowType.MODAL_DIALOG)
          continue;
        if (!win.is_on_all_workspaces() && win.get_workspace() !== active)
          continue;
        const rect = win.get_frame_rect();
        if (rect.width < 2 || rect.height < 2)
          continue;
        list.push({
          rect: { x: rect.x, y: rect.y, w: rect.width, h: rect.height },
          win,
        });
      } catch (_error) {
        // Ignore windows that disappear during enumeration.
      }
    }
    return list.reverse();
  }

  _windowAt(point) {
    for (const window of this._windowList) {
      if (contains(window.rect, point))
        return window;
    }
    return null;
  }

  _pointer(event) {
    const [x, y] = event.get_coords();
    return { x, y };
  }

  _pointInChrome(point) {
    for (const actor of [this._toolbar, this._sizeBadge, this._tips, this._textEntry?.entry]) {
      if (!actor || !actor.visible)
        continue;
      const [x, y] = actor.get_transformed_position();
      const [w, h] = actor.get_transformed_size();
      if (point.x >= x && point.x < x + w && point.y >= y && point.y < y + h)
        return true;
    }
    return false;
  }

  _showTooltip(label, button) {
    if (!this._tooltip)
      return;
    if (!label || !button || this._popup || this._drag) {
      this._tooltip.hide();
      return;
    }
    this._tooltip.set_text(label);
    const [, tw] = this._tooltip.get_preferred_width(-1);
    const [, th] = this._tooltip.get_preferred_height(tw);
    const [bx, by] = button.get_transformed_position();
    const [bw, bh] = button.get_transformed_size();
    let x = Math.round(bx + bw / 2 - tw / 2);
    x = Math.max(8, Math.min(global.stage.width - tw - 8, x));
    let y = Math.round(by - th - 6);
    if (y < 8)
      y = Math.round(by + bh + 6);
    this._tooltip.set_position(x, y);
    this._tooltip.show();
  }

  _setCursor(type) {
    if (this._root?.cursor_type !== type)
      this._root.cursor_type = type;
  }

  _updatePointerCursor(point) {
    if (!this._selection) {
      this._setCursor(Clutter.CursorType.CROSSHAIR);
      return;
    }
    const handle = hitHandle(this._selection, point);
    if (handle) {
      const cursors = {
        nw: Clutter.CursorType.NWSE_RESIZE,
        se: Clutter.CursorType.NWSE_RESIZE,
        ne: Clutter.CursorType.NESW_RESIZE,
        sw: Clutter.CursorType.NESW_RESIZE,
        n: Clutter.CursorType.NS_RESIZE,
        s: Clutter.CursorType.NS_RESIZE,
        w: Clutter.CursorType.EW_RESIZE,
        e: Clutter.CursorType.EW_RESIZE,
      };
      this._setCursor(cursors[handle]);
      return;
    }
    if (contains(this._selection, point)) {
      if (this._tool === Tool.TEXT)
        this._setCursor(Clutter.CursorType.TEXT);
      else if (this._tool === Tool.SELECT)
        this._setCursor(Clutter.CursorType.MOVE);
      else
        this._setCursor(Clutter.CursorType.CROSSHAIR);
      return;
    }
    this._setCursor(Clutter.CursorType.CROSSHAIR);
  }

  _onPress(event) {
    const point = this._pointer(event);
    this._tooltip?.hide();
    const entry = this._textEntry?.entry;
    if (entry) {
      const [ex, ey] = entry.get_transformed_position();
      const [ew, eh] = entry.get_transformed_size();
      const inside = point.x >= ex && point.x < ex + ew &&
        point.y >= ey && point.y < ey + eh;
      if (!inside) {
        this._commitText();
      } else {
        return Clutter.EVENT_STOP;
      }
    }
    if (this._popup) {
      if (this._popupClickAt(point))
        return Clutter.EVENT_STOP;
      this._closePopup();
      return Clutter.EVENT_STOP;
    }
    if (this._pointInChrome(point)) {
      this._toolbar.clickAt(point);
      return Clutter.EVENT_STOP;
    }
    if (this._textEntry) {
      this._commitText();
      return Clutter.EVENT_STOP;
    }
    const button = event.get_button();
    if (button === 2) {
      if (this._selection)
        this._pin();
      return Clutter.EVENT_STOP;
    }
    if (button !== 1) {
      if (this._draft) {
        this._draft = null;
        this._repaint();
      } else if (this._tool !== Tool.SELECT) {
        this._setTool(Tool.SELECT);
      }
      return Clutter.EVENT_STOP;
    }
    const now = GLib.get_monotonic_time() / 1000;
    if (this._lastClick && this._selection &&
      now - this._lastClick.time <= 350 &&
      Math.hypot(point.x - this._lastClick.x, point.y - this._lastClick.y) <= 6 &&
      contains(this._selection, point)) {
      this._lastClick = null;
      this._copy();
      return Clutter.EVENT_STOP;
    }
    this._lastClick = { time: now, x: point.x, y: point.y };
    if (this._tool === Tool.SELECT) {
      if (!this._selection && this._autoDetect && this._hoverWindow) {
        this._drag = {
          mode: 'window',
          anchor: point,
          window: { ...this._hoverWindow },
        };
        return Clutter.EVENT_STOP;
      }
      if (this._selection) {
        const handle = hitHandle(this._selection, point);
        if (handle) {
          this._drag = {
            mode: 'resize',
            handle,
            start: { ...this._selection },
          };
          return Clutter.EVENT_STOP;
        }
        if (contains(this._selection, point)) {
          this._drag = {
            mode: 'move',
            start: { ...this._selection },
            origin: point,
          };
          return Clutter.EVENT_STOP;
        }
      }
      if (this._selection)
        this._clearAnnotations();
      this._drag = { mode: 'new', anchor: point };
      this._selection = { x: point.x, y: point.y, w: 0, h: 0 };
      this._layoutChrome();
      this._repaint();
      return Clutter.EVENT_STOP;
    }

    if (!this._selection || !contains(this._selection, point)) {
      if (this._selection)
        this._clearAnnotations();
      this._drag = { mode: 'new', anchor: point };
      this._selection = { x: point.x, y: point.y, w: 0, h: 0 };
      this._layoutChrome();
      this._repaint();
      return Clutter.EVENT_STOP;
    }

    if (this._tool === Tool.TEXT) {
      this._createTextEntry(point);
      return Clutter.EVENT_STOP;
    }
    this._draft = createAnnotation(this._tool, point, this._style);
    this._drag = { mode: 'annotation' };
    this._repaint();
    return Clutter.EVENT_STOP;
  }

  _onMotion(event) {
    const point = this._pointer(event);
    this._pointerPos = point;
    this._updateMagnifier();
    if (!this._drag)
      this._updatePointerCursor(point);
    if (!this._drag)
      this._toolbar?.updateHover(point);
    else
      this._toolbar?.clearHover();
    if (!this._drag) {
      if (this._autoDetect && !this._selection) {
        const window = this._windowAt(point);
        const rect = window ? window.rect : null;
        if (!sameRect(rect, this._hoverWindow)) {
          this._hoverWindow = rect;
          this._layoutVisuals();
          this._repaint();
        }
      }
      return Clutter.EVENT_PROPAGATE;
    }
    if (this._drag.mode === 'window') {
      if (Math.hypot(point.x - this._drag.anchor.x,
        point.y - this._drag.anchor.y) <= 4)
        return Clutter.EVENT_STOP;
      const anchor = this._drag.anchor;
      this._drag = { mode: 'new', anchor };
      this._selection = rectFromPoints(anchor, point, false);
      this._layoutChrome();
      this._repaint();
      return Clutter.EVENT_STOP;
    }
    if (this._drag.mode === 'new') {
      const shift = Boolean(event.get_state() & Clutter.ModifierType.SHIFT_MASK);
      this._selection = rectFromPoints(this._drag.anchor, point, shift);
      this._layoutChrome();
    } else if (this._drag.mode === 'move') {
      const dx = point.x - this._drag.origin.x;
      const dy = point.y - this._drag.origin.y;
      this._selection = clampSelection({
        x: this._drag.start.x + dx,
        y: this._drag.start.y + dy,
        w: this._drag.start.w,
        h: this._drag.start.h,
      });
      this._layoutChrome();
    } else if (this._drag.mode === 'resize') {
      this._selection = resizeSelection(
        this._drag.start, this._drag.handle, point);
      this._layoutChrome();
    } else if (this._drag.mode === 'annotation' && this._draft) {
      const shift = Boolean(event.get_state() & Clutter.ModifierType.SHIFT_MASK);
      updateAnnotation(this._draft, point, shift);
    }
    this._repaint();
    return Clutter.EVENT_STOP;
  }

  _onRelease(event) {
    if (!this._drag)
      return Clutter.EVENT_PROPAGATE;
    const releasePoint = this._pointer(event);
    const mode = this._drag.mode;
    const windowRect = this._drag.window;
    this._drag = null;
    if (mode === 'window' && windowRect) {
      this._selection = { ...windowRect };
      this._hoverWindow = null;
      this._refreshSelectionPixbuf();
      this._layoutChrome();
      this._repaint();
      this._updatePointerCursor(releasePoint);
      return Clutter.EVENT_STOP;
    }
    if (mode === 'annotation' && this._draft) {
      if (annotationIsValid(this._draft))
        this._addAnnotation(this._draft);
      this._draft = null;
      this._repaint();
    } else if (mode === 'new') {
      if (this._selection && (this._selection.w < 3 || this._selection.h < 3)) {
        this._selection = null;
        this._layoutChrome();
      } else if (this._selection) {
        this._layoutChrome();
        this._refreshSelectionPixbuf();
      }
      this._repaint();
    } else if (mode === 'move' || mode === 'resize') {
      if (this._selection)
        this._refreshSelectionPixbuf();
      this._layoutChrome();
      this._repaint();
    }
    this._updatePointerCursor(releasePoint);
    return Clutter.EVENT_STOP;
  }

  _onScroll(event) {
    if (!this._selection)
      return Clutter.EVENT_STOP;
    const direction = event.get_scroll_direction();
    let delta = 0;
    if (direction === Clutter.ScrollDirection.SMOOTH) {
      const [dx, dy] = event.get_scroll_delta();
      delta = dy !== 0 ? dy : -dx;
    } else if (direction === Clutter.ScrollDirection.UP) {
      delta = 1;
    } else if (direction === Clutter.ScrollDirection.DOWN) {
      delta = -1;
    }
    if (delta === 0)
      return Clutter.EVENT_STOP;
    this._style.width = Math.max(1, Math.min(32,
      this._style.width + Math.sign(delta)));
    this._toolbar.setStyle(this._style.color, this._style.width);
    return Clutter.EVENT_STOP;
  }

  _onKey(event) {
    const symbol = event.get_key_symbol();
    const state = event.get_state();
    const ctrl = Boolean(state & Clutter.ModifierType.CONTROL_MASK);
    const shift = Boolean(state & Clutter.ModifierType.SHIFT_MASK);
    if (this._textEntry)
      return Clutter.EVENT_PROPAGATE;
    if (symbol === Clutter.KEY_Tab) {
      this._autoDetect = !this._autoDetect;
      this._hoverWindow = null;
      this._repaint();
      return Clutter.EVENT_STOP;
    }
    if (symbol === Clutter.KEY_Escape) {
      this._escape();
      return Clutter.EVENT_STOP;
    }
    if (symbol === Clutter.KEY_Return || symbol === Clutter.KEY_KP_Enter) {
      this._defaultOutput();
      return Clutter.EVENT_STOP;
    }
    if (ctrl && symbol === Clutter.KEY_c) {
      this._copy();
      return Clutter.EVENT_STOP;
    }
    if (ctrl && symbol === Clutter.KEY_s) {
      this._save();
      return Clutter.EVENT_STOP;
    }
    if (ctrl && symbol === Clutter.KEY_z && !shift) {
      this._undoAction();
      return Clutter.EVENT_STOP;
    }
    if (ctrl && (symbol === Clutter.KEY_y || (shift && symbol === Clutter.KEY_z))) {
      this._redoAction();
      return Clutter.EVENT_STOP;
    }
    if (ctrl && symbol === Clutter.KEY_a) {
      this._selection = {
        x: 0, y: 0,
        w: global.stage.width, h: global.stage.height,
      };
      this._refreshSelectionPixbuf();
      this._layoutChrome();
      this._repaint();
      return Clutter.EVENT_STOP;
    }
    if (symbol === Clutter.KEY_p && !ctrl) {
      this._pin();
      return Clutter.EVENT_STOP;
    }
    if (this._selection &&
      (symbol === Clutter.KEY_Left || symbol === Clutter.KEY_Right ||
        symbol === Clutter.KEY_Up || symbol === Clutter.KEY_Down)) {
      const step = shift ? 10 : 1;
      const dx = symbol === Clutter.KEY_Left ? -step :
        symbol === Clutter.KEY_Right ? step : 0;
      const dy = symbol === Clutter.KEY_Up ? -step :
        symbol === Clutter.KEY_Down ? step : 0;
      this._selection = clampSelection({
        ...this._selection,
        x: this._selection.x + dx,
        y: this._selection.y + dy,
      });
      this._refreshSelectionPixbuf();
      this._layoutChrome();
      this._repaint();
      return Clutter.EVENT_STOP;
    }
    return Clutter.EVENT_PROPAGATE;
  }

  _escape() {
    if (this._draft) {
      this._draft = null;
      this._repaint();
      return;
    }
    if (this._tool !== Tool.SELECT) {
      this._setTool(Tool.SELECT);
      return;
    }
    this._close();
  }

  _clearAnnotations() {
    this._annotations = [];
    this._undo = [];
    this._redo = [];
    this._draft = null;
  }

  _addAnnotation(annotation) {
    this._annotations.push(annotation);
    this._undo.push(annotation);
    this._redo = [];
  }

  _undoAction() {
    if (this._draft) {
      this._draft = null;
      this._repaint();
      return;
    }
    const annotation = this._undo.pop();
    if (!annotation)
      return;
    const index = this._annotations.lastIndexOf(annotation);
    if (index >= 0)
      this._annotations.splice(index, 1);
    this._redo.push(annotation);
    this._repaint();
  }

  _redoAction() {
    const annotation = this._redo.pop();
    if (!annotation)
      return;
    this._annotations.push(annotation);
    this._undo.push(annotation);
    this._repaint();
  }

  _createTextEntry(point) {
    const entry = new St.Entry({
      style_class: 'cosnip-text-entry',
      can_focus: true,
      reactive: true,
      width: Math.min(360, Math.max(120, this._selection.w - (point.x - this._selection.x))),
    });
    entry.set_style(
      `font-size: ${this._style.fontSize}px; ` +
      `color: rgb(${this._style.color[0]}, ${this._style.color[1]}, ` +
      `${this._style.color[2]});`);
    entry.set_position(Math.round(point.x), Math.round(point.y));
    this._root.add_child(entry);
    this._textEntry = { entry, point };
    entry.grab_key_focus();
    entry.clutter_text.set_cursor_position(0);
    entry.connect('key-focus-out', () => {
      if (this._textEntry?.entry === entry)
        this._commitText();
    });
    entry.connect('key-press-event', (_text, event) => {
      const symbol = event.get_key_symbol();
      if (symbol === Clutter.KEY_Return || symbol === Clutter.KEY_KP_Enter) {
        this._commitText();
        return Clutter.EVENT_STOP;
      }
      if (symbol === Clutter.KEY_Escape) {
        this._cancelText();
        return Clutter.EVENT_STOP;
      }
      return Clutter.EVENT_PROPAGATE;
    });
  }

  _commitText() {
    if (!this._textEntry)
      return;
    const { entry, point } = this._textEntry;
    const text = entry.get_text().trim();
    this._textEntry = null;
    entry.destroy();
    if (text) {
      this._addAnnotation({
        kind: Tool.TEXT,
        x: point.x,
        y: point.y,
        text,
        size: this._style.fontSize,
        color: this._style.color,
        width: this._style.width,
      });
      this._repaint();
    }
    this._root.grab_key_focus();
  }

  _cancelText() {
    if (!this._textEntry)
      return;
    this._textEntry.entry.destroy();
    this._textEntry = null;
    this._root.grab_key_focus();
  }

  _renderSelection() {
    if (!this._selection)
      return null;
    const sel = {
      x: Math.round(this._selection.x),
      y: Math.round(this._selection.y),
      w: Math.max(1, Math.round(this._selection.w)),
      h: Math.max(1, Math.round(this._selection.h)),
    };
    const deviceW = Math.max(1, Math.round(sel.w * this._scale));
    const deviceH = Math.max(1, Math.round(sel.h * this._scale));
    const surface = new cairo.ImageSurface(cairo.Format.ARGB32, deviceW, deviceH);
    const cr = new cairo.Context(surface);
    if (this._selectionPixbuf) {
      paintPixbuf(cr, this._selectionPixbuf, 0, 0, 1);
    } else {
      cr.setSourceRGBA(0, 0, 0, 1);
      cr.paint();
    }
    cr.save();
    cr.scale(this._scale, this._scale);
    cr.translate(-sel.x, -sel.y);
    drawAnnotations(cr, this._annotations, {
      sourcePixbuf: this._selectionPixbuf,
      sourceOrigin: this._selectionPixbufRect ?? sel,
      deviceScale: this._scale,
    });
    cr.restore();
    return surface;
  }

  _defaultOutput() {
    switch (this._settings.get_enum('default-output')) {
      case 1:
        this._save();
        break;
      case 2:
        this._pin();
        break;
      default:
        this._copy();
        break;
    }
  }

  async _copy() {
    await this._ensureSelectionPixbuf();
    const surface = this._renderSelection();
    if (!surface)
      return;
    copySurfaceToClipboard(surface);
    this._notify('Copied selection');
    this._close();
  }

  async _save() {
    await this._ensureSelectionPixbuf();
    const surface = this._renderSelection();
    if (!surface)
      return;
    const dirSetting = this._settings.get_string('save-dir').trim();
    const dir = dirSetting || defaultSaveDir();
    const now = GLib.DateTime.new_now_local();
    const name = expandFilename(this._settings.get_string('filename'), now);
    const path = uniquePath(dir, name);
    try {
      saveSurface(surface, path);
      this._notify(`Saved ${path}`);
    } catch (error) {
      logError(error, 'cosnip: save failed');
      this._notify('Save failed; see log');
      return;
    }
    this._close();
  }

  async _pin() {
    await this._ensureSelectionPixbuf();
    const surface = this._renderSelection();
    if (!surface || !this._selection)
      return;
    this._onPin?.(
      surface,
      Math.round(this._selection.x),
      Math.round(this._selection.y),
      Math.max(1, Math.round(this._selection.w)),
      Math.max(1, Math.round(this._selection.h)));
    this._close();
  }

  _notify(message) {
    try {
      Main.notify('Cosnip', message);
    } catch (_error) {
      log(`cosnip: ${message}`);
    }
  }

  close() {
    this._close();
  }

  _close() {
    if (this._closed)
      return;
    this._closed = true;
    this._closePopup();
    this._tooltip = null;
    this._cancelText();
    if (this._grab) {
      Main.popModal(this._grab);
      this._grab = null;
    }
    if (this._panelBox)
      this._panelBox.visible = this._panelVisible ?? true;
    this._root?.destroy();
    this._root = null;
    this._onClose?.();
  }
}

function sameRect(a, b) {
  if (!a || !b)
    return a === b;
  return a.x === b.x && a.y === b.y && a.w === b.w && a.h === b.h;
}

function contains(rect, point) {
  return point.x >= rect.x && point.y >= rect.y &&
    point.x < rect.x + rect.w && point.y < rect.y + rect.h;
}

function rectFromPoints(a, b, square) {
  let x = Math.min(a.x, b.x);
  let y = Math.min(a.y, b.y);
  let w = Math.abs(b.x - a.x);
  let h = Math.abs(b.y - a.y);
  if (square) {
    const side = Math.max(w, h);
    if (b.x < a.x)
      x = a.x - side;
    if (b.y < a.y)
      y = a.y - side;
    w = side;
    h = side;
  }
  return clampSelection({ x, y, w, h });
}

function clampSelection(rect) {
  const x = Math.max(0, Math.min(global.stage.width, rect.x));
  const y = Math.max(0, Math.min(global.stage.height, rect.y));
  const w = Math.max(0, Math.min(global.stage.width - x, rect.w));
  const h = Math.max(0, Math.min(global.stage.height - y, rect.h));
  return { x, y, w, h };
}

function handlePoints(rect) {
  const x1 = rect.x;
  const y1 = rect.y;
  const x2 = rect.x + rect.w;
  const y2 = rect.y + rect.h;
  return [
    [x1, y1], [x2, y1], [x1, y2], [x2, y2],
    [(x1 + x2) / 2, y1], [(x1 + x2) / 2, y2],
    [x1, (y1 + y2) / 2], [x2, (y1 + y2) / 2],
  ];
}

function hitHandle(rect, point) {
  const positions = [
    'nw', 'ne', 'sw', 'se', 'n', 's', 'w', 'e',
  ];
  const points = handlePoints(rect);
  for (let i = 0; i < points.length; i++) {
    const [x, y] = points[i];
    if (Math.hypot(point.x - x, point.y - y) <= HANDLE_RADIUS)
      return positions[i];
  }
  return null;
}

function resizeSelection(start, handle, point) {
  let left = start.x;
  let top = start.y;
  let right = start.x + start.w;
  let bottom = start.y + start.h;
  if (handle.includes('w'))
    left = point.x;
  if (handle.includes('e'))
    right = point.x;
  if (handle.includes('n'))
    top = point.y;
  if (handle.includes('s'))
    bottom = point.y;
  if (left > right)
    [left, right] = [right, left];
  if (top > bottom)
    [top, bottom] = [bottom, top];
  return clampSelection({
    x: left,
    y: top,
    w: Math.max(1, right - left),
    h: Math.max(1, bottom - top),
  });
}
