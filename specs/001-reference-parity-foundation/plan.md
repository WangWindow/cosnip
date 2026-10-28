# Implementation Plan: Reference Parity Foundation

**Branch**: `001-reference-parity-foundation` | **Date**: 2026-10-29 | **Spec**:
[spec.md](spec.md)

**Input**: Feature specification from `specs/001-reference-parity-foundation/spec.md`

## Summary

Establish a traceable behavioral baseline for Snipaste 2.11.3 Linux and settle the windowing and
capture architecture before product implementation. The work combines static analysis of the
official x86_64 AppImage with a reproducible aarch64 reference runtime and a disposable
`eframe`/XWayland spike. It produces a sanitized research dossier, a parity matrix, measured spike
results, and an architecture decision record. No product crate is created in this feature.

The spike already established the core window strategy: a WM-managed fullscreen X11 window is the
right snip overlay because it receives keyboard focus; pinned images must be override-redirect
ARGB windows with an input shape for click-through. On the target Mali/XWayland setup, transparent
windows require an alpha-capable GL configuration; software GL produced depth-32 pins, while the
default hardware GLX path produced depth-24 pins. This is a product-level risk to carry into the
MVP plan.

## Technical Context

**Language/Version**: Rust stable, edition 2024. MSRV is **1.95** because `eframe`/`egui` 0.37
declare it; the current toolchain is 1.99.1. Both `aarch64-unknown-linux-gnu` and
`x86_64-unknown-linux-gnu` are supported targets.

**Primary Dependencies** (research spike only; product dependencies are decided by later plans):

- `eframe`/`egui` 0.37 (MIT OR Apache-2.0, MSRV 1.95) with `glow`, `x11`, `wayland`,
  `default_fonts`, `system_fonts`.
- `winit` 0.30 (Apache-2.0) for forcing the X11 backend.
- `x11rb` 0.13 (MIT OR Apache-2.0) for X11 capture and XGrabKey probes.
- `arboard` 3 (MIT OR Apache-2.0) for a clipboard image smoke test.
- `image` 0.25 with PNG only (MIT OR Apache-2.0) for output checks.
- Analysis-only: `rea` 6.8.0, `kuna` 1.984, `qemu-x86_64` (QEMU user), `proot` 5.4, Ubuntu
  26.04 amd64 base rootfs, `squashfs-tools`.
- Planned for product planning, not used in this feature: `tiny-skia`, `cosmic-text`, `zbus`,
  `ashpd`, `ksni`.

`wgpu` was evaluated and rejected for this device: the Vulkan device reported
`max_texture_dimension_3d = 512` while egui requested 2048. The glow backend is the spike renderer.

**Storage**: No product database. `temp/` holds the reference AppImage, extracted tree, rootfs,
Kuna exports, qemu/proot tools, and the spike. Only sanitized Markdown/JSON artifacts are
committed under `specs/`, `docs/adr/`, and `docs/parity/`.

**Testing**: Static evidence review, reproducible runtime checklist, X11 window-property and
XShape inspection, measured latency/resource checks, and a manual visual checklist for native
Wayland stacking. No product unit tests are added in this feature.

**Target Platform**: aarch64 Armbian/Ubuntu 26.04, GNOME 49 Wayland with XWayland, 1920x1080,
single monitor. Secondary: x86_64 Linux; X11 and KDE are documented gaps until a later feature.

**Project Type**: Desktop application research/spike; no product source tree yet.

**Performance Goals**: Record, not yet enforce, overlay first-frame latency, pin first-frame
latency, capture latency/status, idle CPU, and RSS on the target. The spike's continuous repaint
inflates CPU and is not a product baseline.

**Constraints**:

- No root/sudo required for analysis; use downloaded static tools and a user-space rootfs.
- The x86_64 reference cannot run natively on aarch64; dynamic observation uses qemu/proot or an
  external x86_64 machine.
- GNOME 49 denies direct `org.gnome.Shell.Screenshot` calls and the portal screenshot path blocks
  on a user decision in this environment; this is recorded as a risk, not hidden.
- Reference artifacts and raw analysis stay in gitignored `temp/`; no proprietary code, strings,
  or assets are committed.
- No product implementation in this feature.

**Scale/Scope**: One reference version, one target device, one disposable spike. Multi-monitor,
mixed-DPI, KDE, and pure-Wayland windowing are explicit gaps.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Personal, Lawful Use Only**: PASS. Official AppImage, personal study, hash verified, no
  license bypass, no redistribution. Reference artifacts remain in `temp/`.
- **II. Behavioral Reference, Original Implementation**: PASS. Only sanitized behavior notes and
  evidence IDs are committed; decompiled output, string dumps, and reference screenshots stay in
  `temp/`. No production code is written.
- **III. Rust Workspace with Isolated Platform Edges**: PASS for a research feature. The spike is
  disposable and in `temp/`; no workspace crate or platform edge is introduced yet. Both
  architectures are documented; `unsafe` is absent from the spike. A single GUI stack
  (`eframe`/`egui`) is used for the spike.
- **IV. Test-First and Evidence-Based Fidelity**: PASS. The feature is evidence-first: every
  behavior claim carries an observation ID; the spike has a pass/fail checklist and measured
  values. Product performance budgets are deferred to the MVP spec.
- **V. Local-Only Privacy and Operational Restraint**: PASS. Analysis is local; network is used
  only to download official tools/reference. No telemetry. Temp artifacts are local and
  gitignored.

No violations. No complexity exception is required.

## Project Structure

### Documentation (this feature)

```text
specs/001-reference-parity-foundation/
├── spec.md
├── plan.md
├── research.md                 # sanitized behavior dossier + evidence IDs
├── data-model.md               # observation/evidence/parity schema
├── quickstart.md               # reproduce reference and spike
├── contracts/
│   ├── evidence.schema.json
│   └── spike-checklist.md
├── checklists/
│   └── requirements.md
└── tasks.md

docs/
├── adr/
│   └── 0001-windowing-capture-architecture.md
├── parity/
│   └── reference-2.11.3.md
└── dependencies.md

temp/                            # gitignored, not committed
├── reference/                    # AppImage + extracted squashfs root
├── analysis/                     # raw kuna/rea output, logs, run scripts
├── qemu/                         # rootfs and qemu tools
├── tools/                        # unpacked helper binaries
└── spike/                        # disposable eframe/XWayland spike
```

### Source Code (repository root)

No `crates/` or `src/` tree is created in this feature. The spike lives only under
`temp/spike/` and is intentionally excluded from Cargo workspace membership. The product
workspace is defined by the next feature's plan.

**Structure Decision**: Keep all research artifacts inside the existing spec-kit structure,
with the ADR and parity matrix under `docs/` so later features can reference them without
depending on this feature's directory. The reference runtime and spike are disposable and
never enter the Cargo workspace.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| None | — | — |

## Phase Plan

### Phase 0 — Reference observation (complete for M0)

1. Acquire and verify the official AppImage; extract the squashfs tree.
2. Inventory the binary with `kuna` and `rea`: ELF shape, Qt version, imports, RTTI/metaobject
   class names, key strings, and targeted decompilation.
3. Map capture backend selection and window strategy; record evidence IDs.
4. Build the qemu/proot reference harness; observe CLI, overlay, toolbar, magnifier, and log
   behavior where the environment permits.
5. Record gaps: GNOME 49 portal permission, native Wayland stacking, multi-monitor/DPI, KDE,
   tray, and global-shortcut delivery.

### Phase 1 — Technical spike (complete)

1. Build a disposable `eframe` 0.37 spike forced to X11.
2. Verify normal fullscreen overlay focus and `override_redirect` pin behavior.
3. Verify ARGB pin depth, input-shape click-through, positioning, taskbar/pager flags, and window
   type.
4. Probe X11 root capture, clipboard set-image, and XGrabKey.
5. Measure first-frame latency and resource use; record that continuous repaint invalidates idle
   CPU as a product baseline.
6. Write the ADR and parity matrix; decide the windowing/capture direction and fallbacks.

### Phase 2 — Handoff

1. Review the dossier and ADR with the project owner.
2. Choose the MVP architecture:
   - Overlay: WM-managed fullscreen X11 window (focus works).
   - Pins: override-redirect ARGB X11 windows; alpha-capable GL configuration required.
   - Capture: compositor/portal backend on Wayland, X11 backend on Xorg; XWayland root capture is
     not a supported capture path.
   - Hotkeys: XGrabKey on X11; portal GlobalShortcuts or desktop shortcut on Wayland.
   - Clipboard: arboard/X11 bridge first; native Wayland helper only if cross-app tests fail.
3. Carry the transparency, portal-permission, and other-compositor stacking risks into the MVP
   plan with explicit acceptance tests.
4. Do not create product crates until the MVP spec/plan is approved.

## Risks and Mitigations

| Risk | Evidence | Mitigation |
|------|----------|------------|
| Transparent pins become depth 24 with hardware GLX | Spike: default GL pin was depth 24; software GL + transparent root produced depth 32 | Investigate EGL/alpha GL config; if unavailable, force software GL for pins or implement an ARGB X11 pin adapter; add depth assertion to the MVP spike |
| GNOME 49 portal screenshot blocks on a user decision | `gdbus` portal call returned a request handle and no response; GNOME Shell D-Bus returned AccessDenied | Test on a real interactive session, inspect portal permission storage, and provide a documented fallback (Xorg or desktop shortcut); do not claim silent capture |
| XWayland root capture fails with BadMatch | Spike `GetImage` on XWayland root failed; reference logs show X11 source but only black content | Do not use XWayland root capture for Wayland sessions; use compositor/portal capture; keep X11 capture for Xorg |
| Native Wayland stacking | Reference pin and overlay visually above native Wayland windows on GNOME 49 (native screencast) | Recheck on KDE/other compositors; keep Xorg as the high-parity fallback |
| Global hotkeys on Wayland | XGrabKey probe succeeded on XWayland root but delivery is not proven; portal GlobalShortcuts untested | Test delivery with a real keypress; provide desktop-shortcut CLI fallback |
| Reference cannot run natively on aarch64 | qemu/proot harness works, but subprocess execution and performance are limited | Use static evidence for behavior; use dynamic observations only for UI/state confirmation; prefer an external x86_64 machine for high-fidelity runs |
| Tray unavailable on GNOME | `org.kde.StatusNotifierWatcher` absent in the session | Treat tray as optional; provide CLI/hotkey access; document AppIndicator extension requirement |

## Dependencies and Licenses (initial)

- `eframe` 0.37, `egui` 0.37, `egui_glow`, `glow`: MIT OR Apache-2.0.
- `winit` 0.30: Apache-2.0.
- `x11rb` 0.13: MIT OR Apache-2.0.
- `arboard` 3: MIT OR Apache-2.0.
- `image` 0.25: MIT OR Apache-2.0.
- `qemu`, `proot`, `squashfs-tools`, Ubuntu base rootfs: analysis-only system tools, not linked
  into Cosnip.
- `rea` and `kuna` are analysis instruments and must never become runtime/build dependencies
  (Constitution II).

## Constitution Re-check (post-design)

The design keeps the work in `temp/` and `specs/`/`docs/`, introduces no product dependency, and
commits only sanitized behavior evidence. The dual-architecture requirement is satisfied by
recording both targets and deferring product build configuration to the MVP plan. No violation.

## Next Actions

1. Review `research.md`, the ADR, and the parity matrix with the project owner.
2. If the go decision is accepted, run `$speckit-specify` for the MVP feature (snipping overlay,
   clipboard/save, and pin) before creating crates.
3. Add a follow-up spike for EGL/alpha GL and the GNOME portal permission path before pin
   implementation.
