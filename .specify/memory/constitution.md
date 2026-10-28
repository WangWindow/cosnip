<!--
Sync Impact Report
==================
Version change: 1.0.0 → 1.1.0 (platform scope amendment)
Modified principles:
  - III. Rust Workspace with Isolated Platform Edges (dual-architecture targets)
Modified sections:
  - Additional Constraints (supported architectures and reference baseline)
Added sections: none
Removed sections: none
Follow-up TODOs: none
-->

# Cosnip Constitution

Cosnip is a personal, non-commercial Rust reimplementation of Snipaste behavior, built for
study and research. The reference application is a behavioral oracle, never a source of
shipped code or assets.

## Core Principles

### I. Personal, Lawful Use Only (NON-NEGOTIABLE)

Cosnip MUST remain a personal study and research project. It MUST NOT be sold, distributed
as a product, or used to provide commercial services.

- The reference build MUST be lawfully obtained. License checks, activation, DRM, and access
  controls MUST NOT be bypassed.
- Original binaries, extracted resources, decompiled output, analysis dumps, and proprietary
  assets MUST remain outside version control and MUST NOT be redistributed.
- The project MUST use its own name, iconography, and visual identity. Snipaste trademarks
  MUST NOT be used to imply endorsement or affiliation.
- If any activity conflicts with applicable law, the Snipaste license, or the terms of an
  analysis tool, work MUST stop until the conflict is resolved.
- Rationale: a learning goal does not grant redistribution or commercial rights.

### II. Behavioral Reference, Original Implementation

Reverse-engineering tools, including rea, are development-time analysis instruments. They
MUST NOT become runtime, build, or test dependencies of Cosnip.

- Observations MUST be recorded as behavior descriptions, diagrams, acceptance criteria, or
  test cases. Decompiled code, string tables, and extracted resources MUST NOT be copied into
  the repository.
- Every committed line of application code MUST be original Rust written for Cosnip. Partial
  copying, mechanical translation, or close paraphrase of proprietary implementation code is
  prohibited.
- Each reverse-engineered behavior MUST be traceable to a recorded observation: reference
  version, steps, inputs, and observed result. Raw analysis artifacts MUST be sanitized before
  any note is committed.
- Analysis MUST target only the locally owned reference copy. It MUST NOT modify the reference
  installation, inject code into unrelated processes, or interfere with other applications.
- Rationale: behavioral compatibility is the goal; provenance and originality are mandatory.

### III. Rust Workspace with Isolated Platform Edges

Cosnip MUST be a standard Cargo workspace using stable Rust, with a thin application binary
and reusable crates under `crates/`.

- Domain logic (geometry, annotation model, capture workflow, history) MUST live in a pure
  Rust core that has no UI, windowing, or operating-system dependencies.
- Platform-specific behavior (screen capture, global hotkeys, clipboard, window pinning,
  tray integration) MUST be isolated behind explicit traits or adapter crates. Linux
  aarch64 and x86_64 are both supported targets; aarch64 is the primary validation and
  release target, while x86_64 remains a supported development and reference-analysis
  host. Application code MUST stay architecture-neutral; architecture-specific
  optimizations MUST have portable fallbacks and MUST be tested on both targets. 32-bit
  ARM and 32-bit x86 are out of scope unless a future amendment adds them explicitly.
- `unsafe` MUST be forbidden by default. Necessary exceptions MUST be isolated, documented
  with a `SAFETY` rationale, covered by tests where feasible, and justified in the plan.
- Every dependency MUST be justified by capability, maintenance status, license
  compatibility, and attack surface. Unmaintained or redundant crates MUST NOT be added.
- One primary GUI stack MUST be selected per plan and used consistently. Mixing UI toolkits
  without an approved plan amendment is prohibited.
- `cargo fmt` and `cargo clippy -- -D warnings` MUST pass for all committed code.
- Rationale: a clean core keeps behavior testable and prevents platform details from leaking
  into the domain model.

### IV. Test-First and Evidence-Based Fidelity

Fidelity MUST be demonstrated by evidence, not by visual resemblance or assumption.

- Each feature MUST begin with acceptance criteria derived from recorded reference behavior
  and a parity entry describing the expected user-visible result.
- Tests MUST be written before or alongside implementation. A bug fix MUST include a
  regression test that fails without the fix.
- Pure domain logic MUST have unit tests. Capture, annotation, export, and persistence flows
  MUST have integration tests. OS-level behaviors that cannot be automated MUST have a
  documented manual parity checklist.
- Rendering comparisons SHOULD use golden images with explicit tolerance for platform and
  font differences. A pixel difference alone MUST NOT be treated as a product defect without
  review.
- Performance budgets MUST be defined per feature and measured. Capture-to-overlay latency,
  input responsiveness, idle memory, and cleanup behavior MUST NOT regress beyond the agreed
  budget.
- A task is not complete until its tests exist and pass, its parity evidence is recorded, and
  its documentation is updated.
- Rationale: measurable parity keeps a reimplementation honest and prevents scope drift.

### V. Local-Only Privacy and Operational Restraint

Cosnip handles screen content, clipboard data, and file paths, which are sensitive by default.

- The core application MUST work fully offline. Telemetry, analytics, crash reporting, and
  automatic network calls are prohibited unless a future amendment explicitly adds an
  opt-in feature.
- Screenshot content, OCR text, clipboard history, and file paths MUST remain local and MUST
  be stored only with user-visible retention controls.
- Logs MUST redact image content, clipboard content, and full file paths. Temporary files
  MUST be cleaned up and created with restrictive permissions.
- Idle CPU, memory, and background activity MUST remain bounded. No persistent helper process
  may be introduced without an approved plan.
- Rationale: a screenshot tool is trusted with everything on screen and must not become a
  data-exfiltration or resource-drain risk.

## Additional Constraints

**Reference baseline**: The Snipaste 2.11.3 Linux x86_64 AppImage is the primary local
reference artifact. The Windows x64/ARM64 builds are optional secondary references and are
not required for Linux feature work. The exact version, hash, and observed behavior MUST be
recorded in the corresponding feature specification. Reference artifacts MUST stay under the
gitignored `temp/` directory and MUST NOT be committed, copied into build outputs, or bundled
with Cosnip.

**Technology**: Stable Rust with an explicitly declared minimum supported Rust version. The
workspace MUST expose a single application binary and separate library crates for domain,
capture, UI, and platform concerns. The release and performance-validation matrix MUST include
both aarch64 and x86_64 Linux builds. Dependency licenses MUST be recorded; a copyleft
dependency requires explicit justification in the plan.

**Assets**: Icons, fonts, sounds, translations, and sample images MUST be original, generated,
or licensed for the project. No Snipaste resource may be reused, traced, or shipped.

**Security**: Cosnip MUST use public operating-system APIs. It MUST NOT require elevated
privileges for normal use, install kernel drivers, hook unrelated processes, or weaken host
security controls.

## Development Workflow

1. **Observe**: Exercise the reference application with rea or manual interaction. Record
   sanitized behavior notes, inputs, outputs, timing, and edge cases.
2. **Specify**: Convert observations into a feature specification with acceptance criteria
   and a parity matrix entry. Ambiguities MUST be resolved before planning.
3. **Plan**: Produce an implementation plan with a Constitution Check. Any deviation MUST be
   documented, justified, and approved before task generation.
4. **Task**: Break work into dependency-ordered tasks that include tests, parity evidence,
   and documentation. Test tasks MUST precede or accompany implementation tasks.
5. **Implement**: Keep changes small and reviewable. Run `cargo fmt`, `cargo clippy -- -D
   warnings`, and `cargo test` before claiming completion.
6. **Verify**: Compare behavior side by side with the reference, update the parity matrix, and
   record any accepted differences with rationale.
7. **Review**: Confirm that no proprietary artifacts, undocumented `unsafe`, unapproved
   dependencies, or telemetry paths were introduced.

The `temp/` directory is local-only. Continuous integration or pre-commit checks MUST fail if
reference binaries, extracted resources, or analysis dumps are tracked.

## Governance

This constitution supersedes other project practices. When a specification, plan, task, or
implementation conflicts with it, the constitution prevails and the conflicting work MUST be
revised.

**Amendments**: An amendment MUST include a written rationale, the affected principles, an
impact analysis, and the required version bump. Amendments take effect only after review and
approval by the project owner, followed by an update to the version and amendment date.

**Versioning policy**: Constitution versions follow semantic versioning.

- MAJOR: removal or backward-incompatible redefinition of a principle or governance rule.
- MINOR: a new principle or section, or materially expanded mandatory guidance.
- PATCH: clarification, wording fix, or non-semantic refinement.

**Compliance review**: Every feature specification, plan, and task list MUST be checked
against this constitution. A plan with an unresolved violation MUST NOT proceed to
implementation. Exceptions MUST be time-boxed, documented, and approved by the project owner.
Complexity beyond the simplest viable design MUST be justified in the plan.

**Version**: 1.1.0 | **Ratified**: 2026-10-27 | **Last Amended**: 2026-10-29
