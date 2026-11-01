#!/usr/bin/env python3
"""Exercise the extension in an isolated GNOME Shell and D-Bus session."""

import ast
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

TESTS = Path(__file__).resolve().parent
EXTENSION = TESTS.parent / "cosnip@choco2233"
SELECTION = (320, 200, 240, 160)


def run(*args, **kwargs):
    return subprocess.run(args, check=True, text=True, **kwargs)


def call(interface, path, method, *args):
    result = run(
        "gdbus",
        "call",
        "--session",
        "--dest",
        interface,
        "--object-path",
        path,
        "--method",
        f"{interface}.{method}",
        "--",
        *(str(arg) for arg in args),
        capture_output=True,
    )
    return result.stdout.strip()


def native(method, *args):
    return call("org.cosnip.Native", "/org/cosnip/Native", method, *args)


def state(method, *args):
    return json.loads(ast.literal_eval(native(method, *args))[0])


def session(root):
    root = Path(root)
    hidpi = os.environ.get("COSNIP_TEST_HIDPI") == "1"
    shell_log = (root / "shell.log").open("w")
    pattern_log = (root / "pattern.log").open("w")
    shell_env = dict(os.environ)
    shell = subprocess.Popen(
        [
            "gnome-shell",
            "--headless",
            "--wayland",
            "--no-x11",
            "--virtual-monitor",
            "2560x1440" if hidpi else "1280x720",
        ],
        stdout=shell_log,
        stderr=shell_log,
        env=shell_env,
    )
    pattern = None
    try:
        for _ in range(100):
            if shell.poll() is not None:
                raise RuntimeError("isolated GNOME Shell exited")
            try:
                native("Ping")
                break
            except subprocess.CalledProcessError:
                time.sleep(0.2)
        else:
            raise RuntimeError("isolated extension did not start")

        pattern_env = dict(
            os.environ, GDK_BACKEND="wayland", WAYLAND_DISPLAY="wayland-0"
        )
        pattern = subprocess.Popen(
            ["gjs", "-m", str(TESTS / "pattern.js")],
            env=pattern_env,
            stdout=pattern_log,
            stderr=pattern_log,
        )
        time.sleep(3)
        schema_dir = root / "data" / "gnome-shell" / "extensions" / EXTENSION.name / "schemas"
        preferences_env = dict(pattern_env, GSETTINGS_BACKEND="memory")
        shell_lib_dirs = [Path("/usr/lib/gnome-shell"), Path("/usr/lib64/gnome-shell")]
        shell_lib_dirs.extend(Path("/usr/lib").glob("*-linux-gnu/gnome-shell"))
        shell_lib = next((path for path in shell_lib_dirs if path.is_dir()), None)
        if shell_lib:
            preferences_env["GI_TYPELIB_PATH"] = os.pathsep.join(filter(None, [
                str(shell_lib / "girepository-1.0"), str(shell_lib),
                preferences_env.get("GI_TYPELIB_PATH"),
            ]))
            preferences_env["LD_LIBRARY_PATH"] = os.pathsep.join(filter(None, [
                str(shell_lib), preferences_env.get("LD_LIBRARY_PATH"),
            ]))
        run("gjs", "-m", str(TESTS / "preferences.js"), str(schema_dir),
            env=preferences_env)
        # Move away from GNOME's top-left hot corner before leaving overview.
        native("TestScroll", 400, 250, 0)
        native("WarpClick", 0, 0, 99)
        time.sleep(1)

        for scale in ((2.0,) if hidpi else (1.0, 1.25)):
            if scale != 1.0:
                interface = "org.gnome.Mutter.DisplayConfig"
                path = "/org/gnome/Mutter/DisplayConfig"
                current = call(interface, path, "GetCurrentState")
                serial = int(re.search(r"uint32 (\d+)", current).group(1))
                mode = re.search(r"'([^']+@[^']+)'", current).group(1)
                monitors = (
                    f"[(0, 0, {scale}, uint32 0, true, [('Meta-0', '{mode}', {{}})])]"
                )
                call(interface, path, "ApplyMonitorsConfig", serial, 1, monitors, "{}")
                time.sleep(2)

            before = root / f"before-{scale}.png"
            preview = root / f"preview-{scale}.png"
            exported = root / f"export-{scale}.png"
            native("CaptureStage", before)
            native("Snip")
            time.sleep(0.4)
            native("TestDrag", 320, 200, 560, 360)
            time.sleep(0.2)
            assert state("DebugState")["selection"] == {
                "x": 320, "y": 200, "w": 240, "h": 160,
            }, "selection drag lost its final geometry"
            native("SetSelection", *SELECTION)
            time.sleep(0.4)
            native("CaptureStage", preview)
            assert native("SaveSelection", exported) == "(true,)"
            run(
                "gjs",
                "-m",
                str(TESTS / "pixels.js"),
                str(before),
                str(preview),
                str(exported),
                str(scale),
                *(str(value) for value in SELECTION),
            )
            if scale == 1.0:
                native("CloseOverlay")
                continue

            debug = state("DebugState")
            button = next(item for item in debug["buttons"] if item["name"] == "shapes")
            native(
                "TestPress",
                round(button["x"] + button["w"] / 2),
                round(button["y"] + button["h"] / 2),
            )
            native("TestPress", 350, 230)
            native("TestMotion", 490, 310)
            time.sleep(0.3)
            drawing = state("DebugState")
            area = drawing["draftBounds"]
            assert area and area["w"] * area["h"] < SELECTION[2] * SELECTION[3], \
                "draft unexpectedly covers the entire selection"
            draft = root / "draft.png"
            committed = root / "committed.png"
            annotated = root / "annotated.png"
            native("CaptureStage", draft)
            native("TestRelease")
            time.sleep(0.3)
            native("CaptureStage", committed)
            assert native("SaveSelection", annotated) == "(true,)"
            run(
                "gjs",
                "-m",
                str(TESTS / "annotation-pixels.js"),
                str(draft),
                str(committed),
                str(annotated),
                str(scale),
                *(str(value) for value in SELECTION),
            )

            def click_button(name, pointer=False):
                button = next(item for item in state("DebugState")["buttons"]
                              if item["name"] == name and item["visible"])
                bx = round(button["x"] + button["w"] / 2)
                by = round(button["y"] + button["h"] / 2)
                if pointer:
                    native("WarpClick", bx, by, 1)
                else:
                    native("TestPress", bx, by)
                time.sleep(0.3 if pointer else 0.15)

            if state("DebugState")["tool"] != "rect":
                click_button("shapes")
            assert state("DebugState")["tool"] == "rect"
            click_button("shapes")
            assert state("DebugState")["tool"] == "select", "second tool click did not cancel the tool"

            def click_at(rect, x_fraction=0.5, y_fraction=0.5, pointer=False):
                x = round(rect["x"] + rect["w"] * x_fraction)
                y = round(rect["y"] + rect["h"] * y_fraction)
                if pointer:
                    native("WarpClick", x, y, 1)
                    time.sleep(0.15)
                else:
                    native("TestPress", x, y)

            # Direct pixel eraser and history.
            click_button("eraser")
            assert len(state("DebugState")["annotationKinds"]) == 1
            native("TestPress", 340, 270)
            native("TestMotion", 360, 270)
            native("TestRelease")
            assert len(state("DebugState")["annotationKinds"]) == 2
            native("WarpClick", 0, 0, 98)
            assert len(state("DebugState")["annotationKinds"]) == 1
            native("WarpClick", 0, 0, 97)
            assert len(state("DebugState")["annotationKinds"]) == 2
            print("direct pixel eraser: undo and redo passed", flush=True)

            native("PinSelection")
            time.sleep(1)
            initial = state("Pins")[0]
            smooth_up = state("TestScroll", 400, 250, -1)[0]
            smooth_down = state("TestScroll", 400, 250, 1)[0]
            wheel_up = state("TestScroll", 400, 250, -10)[0]
            wheel_down = state("TestScroll", 400, 250, 10)[0]
            assert smooth_up["width"] > initial["width"], (initial, smooth_up)
            assert smooth_down["width"] < smooth_up["width"]
            assert wheel_up["width"] > smooth_down["width"]
            assert wheel_down["width"] < wheel_up["width"]
            assert abs(wheel_up["width"] / smooth_down["width"] - math.exp(0.12)) < 0.02
            print("pin zoom: smooth and discrete wheel directions passed", flush=True)

            native("Snip")
            time.sleep(0.4)
            native("SetSelection", *SELECTION)
            time.sleep(0.2)
            click_button("text")
            native("TestPress", 340, 220)
            time.sleep(0.2)
            debug = state("DebugState")
            assert debug["textEntry"], "text entry was not created"
            one_line_height = debug["textEntryHeight"]
            assert debug["inputPanelRaised"], "input panel was not raised above the overlay"
            assert debug["inputPanelAbove"], "input panel is not above the overlay"
            native("TestType", "A\nB")
            time.sleep(0.3)
            debug = state("DebugState")
            assert debug["textEntryText"] == "A\nB", debug
            assert debug["textEntryHeight"] > one_line_height, "multiline entry did not grow"
            native("TestPress", 600, 300)
            native("TestRelease")
            time.sleep(0.2)
            debug = state("DebugState")
            assert not debug["textEntry"], "text entry was not committed"
            assert "A\nB" in debug["texts"], "multiline text annotation was not committed"
            assert not debug["inputPanelRaised"], "input panel was not restored"
            assert not debug["inputPanelAbove"], "input panel was not restored below the overlay"
            native("CloseOverlay")
            print("text input: multiline, IME panel and commit passed", flush=True)

        schema = root / "data" / "gnome-shell" / "extensions" / EXTENSION.name / "schemas"
        schema_id = "org.gnome.shell.extensions.cosnip"

        def set_preference(key, value):
            run("gsettings", "--schemadir", str(schema), "set", schema_id,
                key, repr(value), capture_output=True)

        output_dir = root / "saved-images"
        set_preference("save-dir", str(output_dir))
        set_preference("filename", "saved.png")
        time.sleep(0.3)
        native("Snip")
        time.sleep(0.4)
        native("SetSelection", *SELECTION)
        time.sleep(0.4)
        debug = state("DebugState")
        button = next(item for item in debug["buttons"] if item["name"] == "save")
        native("TestPress", round(button["x"] + button["w"] / 2),
               round(button["y"] + button["h"] / 2))
        saved = output_dir / "saved.png"
        for _ in range(100):
            if saved.exists() and saved.stat().st_size > 0:
                break
            time.sleep(0.1)
        else:
            raise RuntimeError("normal PNG save did not create output")
        print("PNG save verified", flush=True)

        set_preference("save-format", "jpeg")
        set_preference("filename", "saved-jpeg")
        time.sleep(0.3)
        native("Snip")
        time.sleep(0.4)
        native("SetSelection", *SELECTION)
        time.sleep(0.4)
        debug = state("DebugState")
        expected_width = round(SELECTION[2] * debug["outputScale"])
        expected_height = round(SELECTION[3] * debug["outputScale"])
        expected_dpi = debug["dpi"]
        button = next(item for item in debug["buttons"] if item["name"] == "save")
        native("TestPress", round(button["x"] + button["w"] / 2),
               round(button["y"] + button["h"] / 2))
        saved_jpeg = output_dir / "saved-jpeg.jpg"
        for _ in range(100):
            if saved_jpeg.exists() and saved_jpeg.stat().st_size > 0:
                break
            time.sleep(0.1)
        else:
            raise RuntimeError("JPEG save did not create output")
        run("gjs", "-m", str(TESTS / "jpeg.js"), str(saved_jpeg),
            str(expected_width), str(expected_height), str(expected_dpi))
        print("JPEG save verified", flush=True)
        set_preference("save-format", "png")

        run("gsettings", "--schemadir", str(schema), "set", schema_id,
            "save-on-copy", "true", capture_output=True)
        set_preference("filename", "copy-save")
        time.sleep(0.3)
        native("Snip")
        time.sleep(0.4)
        native("SetSelection", *SELECTION)
        time.sleep(0.4)
        debug = state("DebugState")
        button = next(item for item in debug["buttons"] if item["name"] == "copy")
        native("TestPress", round(button["x"] + button["w"] / 2),
               round(button["y"] + button["h"] / 2))
        copied = output_dir / "copy-save.png"
        for _ in range(100):
            if copied.exists() and copied.stat().st_size > 0:
                break
            time.sleep(0.1)
        else:
            raise RuntimeError("copy did not save an image")
        print("copy with save enabled verified", flush=True)
        run("gsettings", "--schemadir", str(schema), "set", schema_id,
            "save-on-copy", "false", capture_output=True)

        blocked = root / "not-a-directory"
        blocked.write_text("occupied")
        set_preference("save-dir", str(blocked))
        set_preference("filename", "retry.png")
        time.sleep(0.3)
        native("Snip")
        time.sleep(0.4)
        native("SetSelection", *SELECTION)
        time.sleep(0.4)
        debug = state("DebugState")
        button = next(item for item in debug["buttons"] if item["name"] == "save")
        bx = round(button["x"] + button["w"] / 2)
        by = round(button["y"] + button["h"] / 2)
        native("TestPress", bx, by)
        for _ in range(100):
            debug = state("DebugState")
            if not debug["saving"]:
                break
            time.sleep(0.1)
        assert debug["selection"] == {"x": 320, "y": 200, "w": 240, "h": 160}, \
            "save failure changed or discarded the selection"
        assert debug["sizeBadge"] != "Saving…", "save failure left the progress state active"
        set_preference("save-dir", str(output_dir))
        time.sleep(0.3)
        debug = state("DebugState")
        button = next(item for item in debug["buttons"] if item["name"] == "save")
        bx = round(button["x"] + button["w"] / 2)
        by = round(button["y"] + button["h"] / 2)
        native("TestPress", bx, by)
        for _ in range(100):
            if (output_dir / "retry.png").exists():
                break
            time.sleep(0.1)
        else:
            debug = state("DebugState")
            actual = run("gsettings", "--schemadir", str(schema), "get", schema_id,
                         "save-dir", capture_output=True).stdout.strip()
            raise RuntimeError(f"saving could not be retried: state={debug}, "
                               f"save-dir={actual}, files={list(output_dir.iterdir())}")
        print("asynchronous save and retry after failure verified", flush=True)
    except Exception:
        shell_log.flush()
        logs = (root / "shell.log").read_text()
        print('\n'.join(line for line in logs.splitlines() if 'cosnip:' in line),
              file=sys.stderr)
        print(logs[-5000:], file=sys.stderr)
        raise
    finally:
        for process in (pattern, shell):
            if process and process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        shell_log.close()
        pattern_log.close()


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "--session":
        session(sys.argv[2])
        return
    with tempfile.TemporaryDirectory(
        prefix="cosnip-headless-", ignore_cleanup_errors=True
    ) as directory:
        root = Path(directory)
        data = root / "data"
        runtime = root / "runtime"
        runtime.mkdir(mode=0o700)
        target = data / "gnome-shell" / "extensions" / EXTENSION.name
        shutil.copytree(EXTENSION, target)
        run("glib-compile-schemas", str(target / "schemas"))

        # The production test hook injects smooth input. Add discrete input to
        # this disposable copy so the same real Clutter event path can test a wheel.
        extension_js = target / "extension.js"
        source = extension_js.read_text()
        smooth = (
            "device.notify_scroll_continuous(GLib.get_monotonic_time(), 0, step,\n"
            "        Clutter.ScrollSource.FINGER, Clutter.ScrollFinishFlags.NONE);"
        )
        discrete = (
            "if (Math.abs(step) > 1)\n"
            "        device.notify_discrete_scroll(GLib.get_monotonic_time(),\n"
            "          step < 0 ? Clutter.ScrollDirection.UP : Clutter.ScrollDirection.DOWN,\n"
            "          Clutter.ScrollSource.WHEEL);\n"
            "      else\n        " + smooth
        )
        if source.count(smooth) != 1:
            raise RuntimeError("cannot install disposable wheel test hook")
        source = source.replace(smooth, discrete)
        overview = "    this._extension.warpClick(x, y, button);"
        if source.count(overview) != 1:
            raise RuntimeError("cannot install disposable overview test hook")
        source = source.replace(
            overview,
            "    if (button === 99) Main.overview.hide();\n"
            "    else if ([95, 96, 97, 98].includes(button)) {\n"
            "      const symbols = {95: Clutter.KEY_Left, 96: Clutter.KEY_Escape,\n"
            "        97: Clutter.KEY_z, 98: Clutter.KEY_z};\n"
            "      const masks = {95: 0, 96: 0,\n"
            "        97: Clutter.ModifierType.CONTROL_MASK | Clutter.ModifierType.SHIFT_MASK,\n"
            "        98: Clutter.ModifierType.CONTROL_MASK};\n"
            "      this._extension._overlay?._onKey({get_key_symbol: () => symbols[button],\n"
            "        get_state: () => masks[button]});\n"
            "    }\n"
            "    else this._extension.warpClick(x, y, button);",
        )
        extension_js.write_text(source)

        env = dict(
            os.environ,
            XDG_DATA_HOME=str(data),
            XDG_CONFIG_HOME=str(root / "config"),
            XDG_RUNTIME_DIR=str(runtime),
            GSETTINGS_BACKEND="keyfile",
            GIO_USE_VFS="local",
        )
        if "--hidpi" in sys.argv[1:]:
            env["COSNIP_TEST_HIDPI"] = "1"
        run(
            "gsettings",
            "set",
            "org.gnome.shell",
            "enabled-extensions",
            "['cosnip@choco2233']",
            env=env,
        )
        try:
            run(
                "dbus-run-session",
                "--",
                sys.executable,
                str(Path(__file__).resolve()),
                "--session",
                str(root),
                env=env,
            )
        finally:
            # Give private portal services a moment to release their FUSE paths.
            time.sleep(1)


if __name__ == "__main__":
    main()
