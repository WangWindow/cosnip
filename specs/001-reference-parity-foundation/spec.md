# Feature Specification: Reference Parity Foundation

**Feature Branch**: `001-reference-parity-foundation`

**Created**: 2026-10-29

**Status**: Draft

**Input**: Reverse engineer the Snipaste 2.11.3 Linux reference behavior and validate the
XWayland windowing and capture architecture on aarch64 before any product implementation.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Traceable Reference Behavior Dossier (Priority: P1)

As a Cosnip implementer, I can look up a precise, sanitized description of a reference
behavior and the evidence that supports it, so I can design and test an original
implementation without copying proprietary code or assets.

**Why this priority**: Every later feature depends on an accurate behavioral baseline. Without
it, requirements become guesses and parity cannot be demonstrated.

**Independent Test**: A reviewer selects any P1 behavior in the parity matrix, follows its
evidence ID, and reaches a recorded observation with version, method, steps, inputs, and
observed result. No proprietary artifact or decompiled code is required to understand it.

**Acceptance Scenarios**:

1. **Given** the reference build is available, **When** the analysis is complete, **Then** each
   P1 behavior in the parity matrix has at least one evidence ID and a verification method.
2. **Given** a raw analysis artifact, **When** the repository is inspected, **Then** the artifact
   is absent from version control and only sanitized descriptions are committed.
3. **Given** a behavior is not observable because of a tool or environment limitation, **When**
   the dossier is reviewed, **Then** the gap is recorded with confidence and a follow-up method.

---

### User Story 2 - Target-Device Feasibility Spike (Priority: P2)

As the project owner, I can run a small, disposable spike on the target aarch64 device and see
whether an XWayland-based overlay and pin window can deliver the essential Snipaste behaviors
on GNOME Wayland.

**Why this priority**: The architecture is only viable if full-screen selection, pin placement,
always-on-top stacking, mouse pass-through, capture, and clipboard bridging actually work on the
target compositor. A failed spike must be discovered before product planning.

**Independent Test**: Run the spike checklist on the device. Each case reports pass, fail, or
blocked with measured values where applicable, and the result is reproducible.

**Acceptance Scenarios**:

1. **Given** GNOME Wayland with XWayland, **When** the spike opens a full-screen overlay, **Then**
   the overlay covers the desktop, receives mouse input, and can be cancelled with Escape.
2. **Given** a native Wayland application is visible, **When** the spike creates a pin window at
   a chosen position, **Then** the pin is frameless, transparent, always-on-top, absent from the
   taskbar, and can toggle mouse pass-through.
3. **Given** the target device, **When** the spike captures the desktop, **Then** it produces an
   image with cursor information where supported and records capture latency.
4. **Given** an image copied by the spike, **When** a native Wayland application pastes it,
   **Then** the image arrives intact; the reverse direction is also tested.
5. **Given** the target session, **When** the spike requests a global shortcut, **Then** it
   records whether XGrabKey, the desktop portal, or a desktop-environment shortcut is usable.

---

### User Story 3 - Approvable Architecture Decision (Priority: P3)

As the project owner, I can approve or reject the recommended windowing/capture architecture
based on evidence, with explicit fallbacks if the preferred approach fails on some compositors.

**Why this priority**: The decision changes the effort, supported desktops, and parity ceiling of
every later feature. It must be recorded, not implied.

**Independent Test**: Read the decision record and check that it states the decision, evidence,
rejected alternatives, supported matrix, fallback path, and residual risks.

**Acceptance Scenarios**:

1. **Given** the spike results, **When** the decision record is written, **Then** it contains a
   clear go/no-go recommendation and the exact conditions that would reverse it.
2. **Given** a compositor where the preferred approach fails, **When** the decision is reviewed,
   **Then** the supported matrix and fallback behavior are explicit.

### Edge Cases

- GNOME Shell screenshot access is denied and the desktop portal requires a user decision.
- The reference build cannot run under emulation because of missing x86_64 userland components.
- The target session has no XWayland, or XWayland cannot stack an override-redirect window above
  native Wayland windows.
- Global shortcuts are unavailable through X11 and no portal implementation exists.
- The reference application requires network access, license activation, or a Pro entitlement
  that is not available; analysis must stop or be scoped to free, offline behavior.
- Only one monitor is available; multi-monitor and mixed-DPI behavior cannot be verified.
- The analysis toolchain cannot decompile a stripped x86_64 binary reliably; static strings and
  behavior observations must be marked with lower confidence.
- A reference behavior would require copying a proprietary asset or code fragment; it is excluded.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The analysis MUST acquire the official Snipaste 2.11.3 Linux x86_64 AppImage,
  verify its published hash, and record the version, hash, size, and acquisition date.
- **FR-002**: The analysis MUST inventory the reference's capture backends, window strategy,
  global-shortcut mechanism, clipboard formats, selection/annotation interaction model, pin
  behavior, output formats, configuration surface, tray behavior, and command-line surface.
- **FR-003**: Every recorded behavior MUST have an evidence ID, reference version/hash, observation
  method, reproduction steps, inputs, observed result, timestamp, and confidence.
- **FR-004**: Raw binaries, decompiled output, string dumps, and reference screenshots MUST remain
  outside version control and MUST NOT be redistributed.
- **FR-005**: The analysis MUST produce a parity matrix that classifies each behavior as P1/P2/P3,
  identifies its verification method, and records gaps.
- **FR-006**: The analysis MUST run a disposable spike on the target aarch64 GNOME Wayland device
  covering full-screen overlay, pin placement/stacking/pass-through, capture, clipboard image
  exchange, and at least one global-shortcut path.
- **FR-007**: The spike MUST record pass/fail/blocked for each case and measure capture-to-overlay
  latency, overlay display latency, pin creation latency, idle CPU, and resident memory.
- **FR-008**: The analysis MUST produce an architecture decision record with a go/no-go
  recommendation, evidence links, rejected alternatives, supported compositor matrix, fallbacks,
  and residual risks.
- **FR-009**: Spike code MUST remain disposable and MUST NOT become a product crate or a build
  dependency.
- **FR-010**: The analysis MUST document how aarch64 and x86_64 builds are expected to stay
  architecture-neutral, and identify any dependency or toolchain assumptions that differ.
- **FR-011**: Any denied, blocked, or unsupported path MUST be reported with an actionable
  workaround or an explicit unsupported status; failures MUST NOT be hidden.
- **FR-012**: No product implementation, annotation engine, installer, or user-facing feature is
  delivered by this feature.

### Key Entities

- **Reference Build**: The verified Snipaste 2.11.3 Linux AppImage and its hash, version, and
  local-only location.
- **Observation**: A recorded interaction or measurement performed against the reference build or
  a target environment.
- **Evidence**: The traceable link between an observation and a behavior claim; includes method,
  reproduction steps, result, timestamp, and confidence.
- **Behavior**: A user-visible or system-visible capability that Cosnip may reimplement.
- **Parity Entry**: A behavior's priority, evidence, verification method, and current status.
- **Spike Case**: A disposable technical check with a pass/fail/blocked result and measurement.
- **Decision**: The architecture choice, its rationale, alternatives, fallbacks, and residual
  risks.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of P1 and P2 behaviors in the parity matrix have at least one traceable evidence
  ID and an explicit verification method.
- **SC-002**: The spike reports a result for all five core cases (overlay, pin/stacking,
  capture, clipboard, shortcut) with zero unlabeled claims; any failure has a fallback.
- **SC-003**: A repository audit finds zero reference binaries, decompiled outputs, string dumps,
  or extracted reference assets under version control.
- **SC-004**: A reviewer can reach a go/no-go decision in one review session using the decision
  record alone, without opening the reference binary or a decompiler.
- **SC-005**: Target-device latency and resource measurements are recorded for overlay display,
  capture, pin creation, idle CPU, and resident memory.
- **SC-006**: The aarch64/x86_64 portability assumptions and any architecture-specific dependency
  risks are documented explicitly.
- **SC-007**: All recorded behaviors are sanitized: no proprietary code, string tables, or assets
  are reproduced in the committed dossier.

## Assumptions

- The reference build is obtained from the official distribution and used only for personal study;
  no redistribution or commercial use is intended.
- GNOME Wayland with XWayland is the primary validation environment; X11 and KDE are secondary and
  may be documented as gaps if unavailable.
- Dynamic reference execution may use emulation and may be slower or less reliable than native
  execution; static evidence is acceptable when dynamic observation is blocked.
- Pro-only behavior, OCR, cloud APIs, and license activation are out of scope.
- Multi-monitor and mixed-DPI behavior may not be reproducible on the current single-monitor
  device; any such gap is recorded for a later feature.
- This feature produces research artifacts, not product code.

## Out of Scope

- Product implementation in `crates/`.
- Annotation editing, pin interactions beyond the spike cases, settings UI, tray UI, packaging,
  localization, OCR, and Pro features.
- Windows and macOS behavior.
- Any attempt to bypass license checks, activation, or access controls.
