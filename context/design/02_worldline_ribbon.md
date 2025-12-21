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

# Document 2 — Worldline Ribbon

## Concept
A clock as a **scrolling ribbon of time**: the present is a cursor; the ribbon moves beneath it. Users can scrub time (without changing system time) to explore DST and offsets.

## Layout
- Full-width horizontal ribbon across the center.
- Above ribbon: selected TZ and offset summary.
- Below ribbon: controls + explanation.

## Ribbon design
- A continuous horizontal band showing:
  - repeating minute markers
  - emphasized hour markers labeled in 12h format
  - second ticks moving smoothly
- The **Now Cursor** is fixed at center (vertical line).
- The ribbon scrolls left-to-right such that “now” aligns at cursor.

## Modes
1) **Live Mode** (default): ribbon moves continuously.
2) **Scrub Mode**: user drags ribbon to inspect a different instant (“ghost time”).
   - Shows ghost timestamp at cursor.
   - A “Return to Now” button appears.

## Components
- `RibbonViewport`
  - `NowCursor`
  - `RibbonTrack` (rendered with transforms)
  - `MarkersLayer` (hours/minutes/seconds)
  - `DstBoundaryLayer`
- `TimeZoneHeader`
- `ScrubControls` (Return to Now, step ±1h, ±1m)
- `TimeZonePicker`

## DST visualization (key feature)
- Overlay a “DST boundary seam” on the ribbon:
  - If a DST transition occurs within visible range (e.g., ±6 hours), show a vertical seam with label:
    - “DST +60m” or “DST −60m”
- If the user scrubs across the seam:
  - show a subtle “time warp” effect (spacing compresses/expands) to hint at the lost/repeated hour.
  - reduced motion: no warp, only a seam + textual note.

## Input
- Mouse/touch drag scrubs.
- Keyboard:
  - `Space` toggles Live/Scrub
  - Arrow left/right steps by 1s
  - Shift+Arrow steps by 1m
  - Ctrl/Cmd+Arrow steps by 1h
- Rotary:
  - rotate steps seconds; press toggles live/scrub.

## States
- `live`
- `scrubbing` with `ghostInstant`
- `transitionHint` when seam enters viewport

## Accessibility
- Cursor has an aria-live region:
  - In Live: announce minute changes (not every second).
  - In Scrub: announce the ghost time on step, not continuously.


---

## Implementation Spec Appendix

### A. Coordinate system + viewport
**Definitions**
- `centerInstant`: instant under the **Now Cursor**.
  - Live mode: `centerInstant = nowInstant`
  - Scrub mode: `centerInstant = ghostInstant`
- `viewportSpanSeconds` (default): `12 * 3600` (±6 hours visible)
- `secondsPerPixel` (default): `30` seconds / px

**Mapping**
- `x = (instant - centerInstant).totalSeconds / secondsPerPixel`
- Cursor is at `x = 0`.

**Zoom levels (optional)**
- Levels: `[5, 10, 30, 60, 120]` seconds/px
- Keyboard: `Ctrl/Cmd + +` zoom in, `Ctrl/Cmd + -` zoom out.

### B. Tick rendering rules
Render ticks for instants in `[centerInstant - span/2, centerInstant + span/2]`.

**Tick cadence**
- Major: every hour
- Medium: every 5 minutes
- Minor: every minute
- Second ticks: only within ±90 seconds of cursor (performance-friendly), unless Reduced Motion.

**Labeling**
- Hour label: `h:mm AM/PM` at hour tick.
- Date label at midnight crossing: e.g., `Mon Jan 12`.

### C. Time zone + DST seam calculation
For selected tz:
- Query transitions in `[centerInstant - 7 days, centerInstant + 7 days]`.
- Filter to instants inside viewport.

Each transition:
- `transitionInstantUtc`
- `deltaMinutes`
- `localWallTimeBefore`, `localWallTimeAfter` (best effort)

**Seam rendering**
- Vertical line at `x(transitionInstantUtc)`.
- Label: `DST +60m` or `DST −60m`, plus local transition timestamp.

### D. Warp effect spec (deterministic)
Warp is purely visual.

Let:
- `W = 30 minutes` (warp half-width) = `1800 seconds`
- `d = (instant - transitionInstantUtc).seconds`

If `|d| > W` → no warp.

Define:
- `u = (d + W) / (2W)` in `[0..1]`
- `smoothstep(u) = u*u*(3 - 2u)`
- `jumpPx = (deltaMinutes * 60) / secondsPerPixel`
- `warpPx = jumpPx * (smoothstep(u) - 0.5)`

Apply:
- `xWarped = x + warpPx`

**Reduced Motion:** disable warp; render seam + textual note only.

### E. State machine
States:
- `LIVE`
- `SCRUB` (with `ghostInstant`)
- `PICKER_OPEN`
- `HELP_OPEN` (optional)

Transitions:
- Drag ribbon → `SCRUB` (set `ghostInstant` from drag delta)
- Click “Return to Now” → `LIVE`
- `Space` toggles `LIVE <-> SCRUB` (entering SCRUB seeds `ghostInstant = nowInstant`)
- TZ picker overlays without changing LIVE/SCRUB.

### F. Input mapping (precise)
**Drag**
- Start: store `dragStartX`, `dragStartInstant = centerInstant`
- Move: `deltaSeconds = -(currentX - dragStartX) * secondsPerPixel`
- `ghostInstant = dragStartInstant + deltaSeconds`

**Keyboard**
- `Space`: toggle LIVE/SCRUB
- `ArrowLeft/Right`: ±1s (in SCRUB; in LIVE, temporarily enters SCRUB)
- `Shift + ArrowLeft/Right`: ±1m
- `Ctrl/Cmd + ArrowLeft/Right`: ±1h
- `Esc`: close overlays

**Rotary**
- Rotate: ±1s (or ±1m if coarse step control focused)
- Press: toggle LIVE/SCRUB

### G. Focus order
1) TZ summary button
2) Ribbon viewport
3) Return-to-Now (only in SCRUB)
4) Step controls (±1h, ±1m)
5) DST info button/card
6) Favorites (if present)

### H. Acceptance criteria
- Cursor timestamp equals `format(localDateTime(centerInstant))`.
- Switching TZ does not change `centerInstant` (same instant, different local representation).
- DST seam appears at correct instant; label matches `deltaMinutes`.
- Scrubbing is stable: no drift; LIVE snaps to `nowInstant`.

