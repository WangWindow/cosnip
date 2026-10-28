# Tasks: Reference Parity Foundation

**Input**: `specs/001-reference-parity-foundation/` design documents
**Prerequisites**: spec.md, plan.md, research.md, data-model.md, contracts/
**Feature type**: Research/spike; no product source tree is created.

## Phase 1: Setup

- [x] T001 Create `temp/reference`, `temp/analysis`, `temp/tools`, `temp/qemu`, and `temp/spike`
- [x] T002 Download the official Snipaste 2.11.3 AppImage and verify SHA-1
- [x] T003 Acquire `squashfs-tools`, `qemu-user`, `proot`, and the Ubuntu amd64 base rootfs under `temp/`
- [x] T004 Scaffold the disposable `eframe` 0.37 spike under `temp/spike/` (excluded from the Cargo workspace)

---

## Phase 2: Foundational

- [x] T005 Amend `.specify/memory/constitution.md` to v1.1.0 (aarch64 + x86_64)
- [x] T006 Extract the AppImage squashfs tree and record the build metadata in `research.md`
- [x] T007 Inventory the main ELF, Qt version, plugins, imports, and RTTI/metaobject classes
- [x] T008 Build the qemu/proot reference harness and verify X11/D-Bus access
- [x] T009 Configure `REA_KUNA_COMMAND` and confirm kuna/provider health
- [x] T010 Create Feature 001 spec, plan, checklist, and `.specify/feature.json`

**Checkpoint**: The reference runs and the analysis toolchain is usable.

---

## Phase 3: User Story 1 - Traceable Reference Behavior Dossier (P1)

**Goal**: Every P1/P2 behavior has sanitized evidence and a verification method.

- [x] T011 [US1] Run `kuna functions --summary` and targeted `kuna strings`/`xrefs` on the main binary
- [x] T012 [US1] Decompile the backend factory, window factories, clipboard helpers, and hotkey probe
- [x] T013 [US1] Map RTTI/metaobject classes for annotation, pin, capture, and clipboard
- [x] T014 [US1] Extract the CLI, default hotkeys, config path, history, and tray behavior
- [x] T015 [US1] Write the sanitized dossier in `specs/001-reference-parity-foundation/research.md`
- [x] T016 [US1] Define the evidence/observation schema and data model
- [x] T017 [US1] Populate `docs/parity/reference-2.11.3.md`
- [x] T018 [US1] Verify raw analysis artifacts remain under `temp/` and are not tracked
- [x] T019 [US1] Record gaps and confidence levels in `research.md` and the parity matrix

**Checkpoint**: The dossier and parity matrix are reviewable without the reference binary.

---

## Phase 4: User Story 2 - Target-Device Feasibility Spike (P2)

**Goal**: Prove or disprove the XWayland window/capture strategy on aarch64.

- [x] T020 [US2] Run the reference under qemu/proot and verify the single-instance CLI
- [x] T021 [US2] Force X11 and capture the reference overlay, toolbar, tips, and magnifier
- [x] T022 [US2] Verify `--hold`, `--area`, destination behavior, and the second-instance trigger
- [x] T023 [US2] Probe GNOME Shell Screenshot and the freedesktop portal; record AccessDenied/blocked
- [x] T024 [US2] Spike WM-managed fullscreen overlay: geometry, focus, first-frame latency
- [x] T025 [US2] Spike override-redirect ARGB pin: position, type, taskbar/pager, depth
- [x] T026 [US2] Verify click-through via an empty X11 input shape
- [x] T027 [US2] Probe X11 root capture, clipboard image set, and XGrabKey
- [x] T028 [US2] Record latency, CPU, and RSS; identify continuous-repaint inflation
- [x] T029 [US2] Compare hardware GLX vs software GL for transparent pin depth
- [ ] T030 [US2] Manual visual check: pin above a native Wayland window on GNOME
- [ ] T031 [US2] Follow-up spike: EGL/alpha GL configuration for depth-32 pins without software GL
- [ ] T032 [US2] Follow-up: interactive GNOME 49 portal capture and permission UX
- [ ] T033 [US2] Build/run the spike on x86_64 to confirm architecture neutrality
- [ ] T034 [US2] Multi-monitor and mixed-DPI coordinate test
- [ ] T035 [US2] KDE/KWin ScreenShot2 integration test
- [ ] T036 [US2] Clipboard image paste into and out of a native Wayland application

**Checkpoint**: Core cases pass, with explicit blocked/follow-up items.

---

## Phase 5: User Story 3 - Approvable Architecture Decision (P3)

**Goal**: A reviewable go/no-go decision with fallbacks.

- [x] T037 [US3] Write `docs/adr/0001-windowing-capture-architecture.md`
- [x] T038 [US3] Record spike results in `contracts/spike-checklist.md`
- [x] T039 [US3] Record the initial dependency/license table in `docs/dependencies.md`
- [ ] T040 [US3] Project-owner review and go/no-go sign-off

**Checkpoint**: The decision record is ready for the review gate.

---

## Phase 6: Polish and Handoff

- [x] T041 Update `quickstart.md` with reproducible commands and expected results
- [x] T042 Audit `git status` for reference binaries, raw analysis, and generated artifacts
- [ ] T043 Commit the constitution, spec, research, ADR, parity matrix, and dependency record
- [ ] T044 After M0 approval, create the MVP feature with `$speckit-specify`

## Dependencies

- US1 depends on Setup and Foundational tasks.
- US2 depends on the reference harness (T008) and can proceed alongside US1's dossier.
- US3 depends on US1 and US2 results.
- T030–T036 are follow-ups and may become tasks in the MVP feature if they block it.

## Notes

- `temp/` is intentionally gitignored; do not commit reference artifacts.
- No product tests or source files are added by this feature.
- The spike is disposable and must not become a Cargo workspace member.
