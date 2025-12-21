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

# Document 4 — Chrono-Superposition

## Concept
A clock that treats time zones as **simultaneous realities**. Multiple time zones are shown at once in a superposed “deck,” which collapses into a composite readout when focused.

## Layout
- Center: “Superposition Core” layered time cards (deck).
- Left: “Zone Field” (search + toggles).
- Right: “Collapse Controls” (focus strength, compare mode).

## Superposition Core behavior
Each selected zone is a card showing:
- time (12h with seconds)
- date
- offset + DST badge

The top card is the “dominant” zone (last clicked).

## Collapse mechanic
- Slider “Focus Strength”:
  - Low: cards remain spread.
  - High: cards collapse into one composite readout that blends differences:
    - seconds aligned
    - hours shown as a range if they differ
    - date shows “same day” or two dates if crossing midnight

## Compare mode
Toggle “Compare Dominant vs Others”:
- Dominant stays crisp.
- Others show delta badges: `+3h`, `−8h`, `+1 day`, `DST differs`.
- Clicking a card swaps dominance.

## DST emphasis
- If any zone has a DST transition within 24h:
  - that card emits a warning motif
  - compare mode shows: “DST shift in 6h (+60m)”

## Input
- Keyboard:
  - Tab navigates cards
  - Arrow up/down changes dominant card
  - `C` toggles compare
  - `F` focuses search
- Rotary:
  - rotate cycles dominant; press toggles compare.

## Accessibility
- Provide a “List Mode” toggle that renders an equivalent semantic list.


---

## Implementation Spec Appendix

### A. Data model
- `selectedZones: ZoneId[]` (1..N)
- `dominantZoneId: ZoneId`
- `focusStrength: 0..1`
- `compareMode: boolean`
- `listMode: boolean` (manual toggle + automatic fallback)

### B. Card ordering (deterministic)
Sort into `displayZones` by:
1) Dominant first
2) Favorites next (stable)
3) Remaining by absolute UTC offset ascending, then tz name (lex)

Auto-enable `listMode` when `N > 8` (allow override via “Show Deck Anyway”).

### C. Card layout geometry
- Deck offset per card `i`:
  - translate `(i*10px, i*8px)`
  - rotation `(i*0.6°)` alternating sign
- Parallax (optional):
  - pointer move adds translate: `clamp(pointerDelta,-1..1) * (1 + i*0.15) * 6px`

Reduced motion: disable parallax and rotations; keep stacking translate only.

### D. Composite readout rules (precise)
When `focusStrength >= 0.8` and not listMode, render composite.

For each zone, compute local:
- `localHour12`, `localHour24`, `localMinute`, `localSecond`, `meridiem`, `localDate`

Display minutes/seconds once: `mm:ss`.

Meridiem:
- If all same → single AM/PM
- Else → `AM–PM`

Compute sortable:
- `wallMinutes = localDayIndex*1440 + localHour24*60 + localMinute`
- `localDayIndex` is date difference to dominant date: `-1, 0, +1`

Let endpoints be min and max `wallMinutes` zones.

Cases:
1) All zones same date and same hour12/meridiem:
   - `h:mm:ss AM/PM`
2) Same date, different hours, same meridiem:
   - `hMin–hMax:mm:ss AM/PM`
3) Spans meridiem or date:
   - show endpoints with meridiem and date badges (`Yesterday|Today|Tomorrow` relative to dominant).

Compare mode deltas:
- `Δhours = round((zoneOffsetMinutes - dominantOffsetMinutes)/60)`
- `Δdays` from date difference
- `DST differs` if `isDst` differs.

### E. Keyboard + focus navigation
Focus regions:
1) Zone Field
2) Core Deck
3) Collapse Controls

Bindings:
- `ArrowUp/Down` (Core focused): move dominance in `displayZones`
- `Enter` on card: set dominant
- `C`: toggle compare
- `L`: toggle list mode
- `F`: focus zone search

Rotary:
- rotate cycles dominance
- press toggles compare (or activates focused control)

### F. State machine
- `DECK_VIEW`
- `COMPOSITE_VIEW` (focusStrength >= 0.8 and not listMode)
- `LIST_VIEW` (listMode)
- `PICKER_OPEN`

### G. Acceptance criteria
- Composite endpoints equal min/max wallMinutes across zones.
- Dominant zone always present and first.
- List mode triggers at N>8 but is overridable.

