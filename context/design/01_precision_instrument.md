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

# Document 1 — Precision Instrument

## Concept
A clock as a **calibrated instrument panel**: crisp typography, grid-aligned readouts, and a secondary “calibration ring” that visualizes seconds.

## Layout
- Responsive container, centered, max width ~ 900px.
- Two-column grid at wide widths; single-column stack below ~ 640px.

### Left panel (Primary Time)
- Huge time: `hh:mm:ss` (12-hour), with smaller `AM/PM`.
- Date line: `Weekday, Month Day, Year`.
- TZ line: `TZ Name (Abbrev) · UTC±hh:mm · DST On/Off`.

### Right panel (Controls + Diagnostics)
- Time zone picker (search + results list).
- Favorites row (chips).
- DST card (status + next/last transition).
- “Time signal” visualization: circular ring + tick mark sweeping once per minute.

## Components (suggested structure)
- `ClockShell`
  - `PrimaryReadout`
  - `CalibrationRing`
  - `TimeZonePicker`
  - `DstStatusCard`
  - `FavoritesChips`
  - `ErrorBanner` (conditional)

## Interaction
- Clicking/tapping the TZ line opens picker.
- Picker overlay:
  - search field autofocus
  - results list virtualized if large
  - selecting sets `selectedTimeZoneId`
- Favorites:
  - star in results adds/removes favorite
- Calibration ring:
  - hover shows tooltip: “System tick: 60fps / Display: 1s”
  - reduced motion: ring becomes static with a highlighted second tick

## Visual rules
- Typography: monospaced for time; proportional for labels.
- Strong contrast.
- Subtle glow on current time digits (optional).
- Ring: 60 tick marks; highlight the current second.

## States
- `normal`
- `pickerOpen`
- `tzMissing/tzDataStale` → show banner + force UTC fallback option

## DST details
- If `dstChange.upcoming` within 24h, show warning with absolute time:
  - “DST shift +60m at Mar 10, 2026 02:00 AM (local).”
- If `justOccurred`, show informational note.

## Accessibility
- Time readout has an accessible label that reads naturally:
  - “It is 9 41 and 32 seconds PM, Pacific Time.”
- Picker supports full keyboard selection and announces result count.

