# Data Model: Reference Parity Foundation

This feature does not persist user data. It defines the research records that later feature
specifications consume. Records are committed only after sanitization; raw artifacts stay in
`temp/`.

## ReferenceBuild

| Field | Type | Notes |
|---|---|---|
| `id` | string | Stable identifier, e.g. `snipaste-2.11.3-linux-x86_64` |
| `version` | string | `2.11.3` |
| `platform` | string | `linux-x86_64` |
| `sha1` | string | Published hash, verified locally |
| `size_bytes` | integer | 37,450,944 |
| `source` | string | Official distribution URL (documented, not bundled) |
| `acquired_at` | date | ISO date |
| `local_path` | string | `temp/reference/...`, never committed |
| `notes` | string | Qt version, packaging observations |

## Observation

| Field | Type | Notes |
|---|---|---|
| `id` | string | `OBS-###` |
| `reference_build_id` | string | Links to `ReferenceBuild` |
| `method` | enum | `static`, `dynamic`, `manual`, `measurement` |
| `tool` | string | e.g. `rea`, `kuna`, `gdbus`, `spike`, `manual` |
| `timestamp` | datetime | ISO 8601 |
| `steps` | string[] | Reproduction steps |
| `inputs` | object | Sanitized inputs only |
| `observed` | string | Behavior or measurement |
| `expected` | string? | Optional hypothesis |
| `confidence` | enum | `high`, `medium`, `low` |
| `evidence_ids` | string[] | Cross-links |
| `sanitized` | boolean | Must be true for committed records |
| `limitations` | string[] | Environment/tool constraints |

## Behavior

| Field | Type | Notes |
|---|---|---|
| `id` | string | `BEH-###` |
| `category` | enum | capture, overlay, annotation, pin, clipboard, hotkey, tray, output, config, cli |
| `title` | string | Short behavior name |
| `description` | string | Testable behavior description |
| `priority` | enum | `P1`, `P2`, `P3` |
| `parity_status` | enum | `observed`, `partial`, `gap`, `deferred` |
| `verification` | string | How Cosnip will verify parity |
| `evidence_ids` | string[] | Observation evidence |

## ParityEntry

| Field | Type | Notes |
|---|---|---|
| `behavior_id` | string | Foreign key to Behavior |
| `reference_behavior` | string | Sanitized description |
| `reference_evidence` | string[] | Observation IDs |
| `cosnip_status` | enum | `not-started`, `in-progress`, `parity`, `intentional-difference`, `blocked` |
| `test_kind` | enum | `unit`, `integration`, `golden`, `manual` |
| `notes` | string | Accepted differences, blockers |

## SpikeCase

| Field | Type | Notes |
|---|---|---|
| `id` | string | `SPIKE-###` |
| `name` | string | Overlay/pin/capture/clipboard/hotkey |
| `environment` | object | OS, compositor, renderer, display count, scale |
| `result` | enum | `pass`, `fail`, `blocked` |
| `metrics` | object | Latencies, depth, geometry, CPU, RSS |
| `artifacts` | string[] | Temp-only paths; not committed |
| `limitations` | string[] | Emulation, permissions, unavailable hardware |

## Decision

| Field | Type | Notes |
|---|---|---|
| `id` | string | ADR number |
| `status` | enum | `proposed`, `accepted`, `rejected`, `superseded` |
| `decision` | string | Chosen architecture |
| `context` | string | Problem and constraints |
| `evidence` | string[] | Observation/spike IDs |
| `alternatives` | string[] | Rejected options and why |
| `fallbacks` | string[] | Per-compositor/GL fallback |
| `risks` | string[] | Residual risks |

## Gap

| Field | Type | Notes |
|---|---|---|
| `id` | string | `GAP-###` |
| `area` | string | Capture, stacking, scaling, tray, etc. |
| `impact` | string | Why it matters |
| `confidence` | enum | `high`, `medium`, `low` |
| `follow_up` | string | Concrete next step |
| `owner` | string | `feature-002` or later |

## Relationships

```text
ReferenceBuild 1 ── * Observation * ── * Behavior 1 ── 1 ParityEntry
                                  \
                                   * SpikeCase
Decision * ── * Observation/SpikeCase
Gap * ── * Behavior/SpikeCase
```

## Validation Rules

- A committed `Observation` MUST have `sanitized = true`.
- A `ParityEntry` MUST reference at least one evidence ID to be considered observed.
- A `Decision` MUST cite at least one spike case and list at least one fallback when a case is
  `fail` or `blocked`.
- No `local_path` may point to a committed file; it must point under `temp/`.
- A `SpikeCase` result MUST be one of `pass`, `fail`, `blocked`; no implicit pass.
