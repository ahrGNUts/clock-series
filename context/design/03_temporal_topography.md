# Global Time Engine Contract (used by all documents)

This contract defines the shared data model, ticking, time zone behavior, DST handling, input support, and accessibility expectations that every clock design in this series relies on.

## Time sources
- **System clock**: current instant `nowInstant` from OS/runtime.
- **Time zone database**: **IANA tz identifiers** (e.g., `America/Los_Angeles`) + offset/DST rules.
- **Formatter**: locale-aware (default English).

## Required outputs per render tick
Given `selectedTimeZoneId` and `nowInstant`, compute:
- `localDateTime`: year, month, day, weekday, hour12, minute, second
- `meridiem`: AM/PM
- `utcOffsetMinutes` (e.g., -480)
- `isDst`: boolean
- `dstChange`: one of:
  - `none`
  - `upcoming` with `{instant, deltaMinutes}`
  - `justOccurred` with `{instant, deltaMinutes}`
- `tzAbbrev` (best-effort; may be empty)
- `validity`: `ok | tzMissing | tzDataStale | unknown`

## Tick strategy
- Update **at least 10 fps** for smooth animations, but *display* seconds as integers.
- Derive displayed seconds from `nowInstant`, not from an incrementing counter (prevents drift).
- If the app is backgrounded, allow coarse ticking and **resync on focus**.

## Time zone selection + DST behavior (must exist in every design)
- A **time zone picker** with:
  - search by city/region text
  - favorites/star list
  - “Use system time zone” shortcut
- A **DST indicator**:
  - shows whether DST is active
  - warns if DST change is within next 24 hours (or just occurred within last 24 hours)

### Edge cases
- If tz data missing: show fallback UTC clock + error banner.
- If user picks an invalid tz id: revert to last valid and show toast.

## Input support (every design)
- Mouse, touch, keyboard (**required**). Rotary/remote (**supported**, lower priority).

### Keyboard baseline
- `Tab` navigates focusable elements
- Arrow keys adjust focused control (where relevant)
- `Enter/Space` activates
- `Esc` closes overlays
- `/` focuses search (if present)

### Rotary baseline
- rotate = next/prev item in focused list (or step value)
- press = select/confirm

## Accessibility baseline
- Every interactive element has programmatic name/role/state.
- Visible focus ring.
- Minimum target size ~ 40px (desktop can be slightly smaller but keep consistent).
- Provide **Reduced motion** mode: disables continuous animations, keeps discrete transitions.


---

# Document 3 — Temporal Topography

## Concept
A clock as a **topographic map of the day** where elevations represent “temporal intensity.” You read time by locating yourself on the terrain.

## Layout
- Main canvas: “Day Map” filling most area.
- Side panel: explicit digital time (for grounding) + timezone + DST + legend.

## Day Map rules
- Horizontal axis = local day (handles 23/24/25-hour days).
- Vertical axis = “temporal elevation” computed deterministically from hour/min/sec.
- A **Locator Beacon** marks the current time and pulses each second.

## DST representation
- The map includes a **fault line** at the DST transition time (if any today in selected TZ):
  - Spring forward: a missing segment (gap) for skipped local hour.
  - Fall back: an overlapped segment (double-labeled region).

## Components
- `DayMapCanvas`
  - `TerrainLayer`
  - `GridLayer` (hours)
  - `FaultLineLayer`
  - `LocatorBeacon`
- `SidePanel`
  - `ExplicitTimeReadout`
  - `TimeZonePicker`
  - `DstStatusCard`
  - `Legend` (how to read map)

## Interaction
- Hover/tap map shows tooltip time at cursor.
- Click selects a point -> “inspect mode”:
  - Side panel shows inspected time and whether it’s ambiguous/nonexistent (DST).
  - “Return to now” exits inspect mode.

## Input
- Keyboard:
  - Arrow keys move inspection cursor by minute
  - Shift+Arrow by hour
  - Enter pins/unpins inspection
- Touch:
  - drag moves inspection cursor
  - double-tap return to now
- Rotary:
  - rotate moves inspection cursor; press pins.

## Accessibility
- Provide a non-visual “Map Summary” describing current position in deterministic language.


---

## Implementation Spec Appendix

### A. Day map domain
Let `tLocal` be local time in selected tz.

Compute:
- `secondsSinceLocalMidnight` (SSM) for `nowInstant` interpreted in tz.
- `dayLengthSeconds = secondsBetween(localMidnight, nextLocalMidnight)` via tz rules.
- Normalize: `p = SSM / dayLengthSeconds` in `[0..1]`.

Horizontal:
- `x = p * canvasWidth`

### B. Deterministic terrain function
Inputs:
- `h = hour12` in `1..12`
- `m = minute` in `0..59`
- `s = second` in `0..59`
- `d = dayOfYear` in `1..366`

Normalized:
- `H = h / 12`
- `M = m / 60`
- `S = s / 60`
- `D = d / 366`

For each sample point at normalized `p`:
```
e(p) =
  0.50 * sin(2π * (p + H)) +
  0.25 * sin(2π * (4p + M)) +
  0.15 * sin(2π * (16p + S)) +
  0.10 * sin(2π * (p + D))
```
Clamp `e(p)` to `[-1..1]`.

Vertical mapping:
- `y = midY - e(p) * amplitudePx`
- Default: `amplitudePx = 0.35 * canvasHeight`

Sampling density:
- `N = max(240, canvasWidth)` samples.

### C. Grid + labeling
- Major hour lines at each local hour boundary.
- Minor lines every 15 minutes.

DST day behavior:
- Spring-forward: skipped hour boundary absent; show “gap label”.
- Fall-back: repeated hour boundaries duplicated with suffix:
  - `1:00 AM (A)` and `1:00 AM (B)`

Tooltip and inspection snapping:
- Snap to nearest minute:
  - `snappedSeconds = round(SSM / 60) * 60`

### D. DST fault line rendering
If a DST transition occurs within local day:
- `transitionP = SSM(transitionLocalInstant) / dayLengthSeconds`
- Fault line at `x = transitionP * width`

**Spring forward (+60)**
- Gap band:
  - `gapWidthPx = (deltaMinutes*60 / dayLengthSeconds) * width`
- Gap extends right from fault line by `gapWidthPx`.
- Terrain not drawn in gap; dashed connectors at edges.

**Fall back (−60)**
- Overlap band (same width magnitude).
- Draw two faint traces inside overlap:
  - Trace A = before transition mapping
  - Trace B = after transition mapping
- Label: “Repeated hour”.

Reduced motion: no beacon pulsing; keep static markers.

### E. Locator beacon
- `pNow = SSM(now) / dayLengthSeconds`
- `xNow = pNow * width`
- `yNow = terrainY(xNow)`

Pulse timing:
- `350ms`: scale 1.0 → 1.4 → 1.0
- Reduced motion: toggle outline for `200ms` at second boundary.

### F. State machine
- `LIVE`
- `INSPECTING` (with `inspectSSM`)
- `PICKER_OPEN`

Transitions:
- Click/tap map → `INSPECTING`
- Escape / “Return to Now” → `LIVE`

### G. Focus order
1) Explicit time readout (focusable for SR)
2) TZ picker button
3) Map canvas region
4) Return to Now (only in INSPECTING)
5) Legend toggles

### H. Acceptance criteria
- Terrain y-values match exactly given same inputs.
- DST band/gap width corresponds to `deltaMinutes` and aligns to transition time.
- Nonexistent minutes (spring-forward gap) are flagged as nonexistent; ambiguous minutes flagged as ambiguous.

