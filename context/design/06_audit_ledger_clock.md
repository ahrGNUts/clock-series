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

# Document 6 — Audit Ledger Clock

## Concept
A clock as an **event ledger**: each second is an entry; minutes are blocks; hours are chapters. “Now” becomes a navigable log.

## Layout
- Main: vertical ledger stream (newest at top).
- Sidebar: zone picker + filters + DST insights.
- Header: current time readout with a conceptual “verification stamp.”

## Ledger rules
Every second creates a row:
- timestamp (12h with seconds)
- `blockId` = current minute
- `chapterId` = current hour
- offset + DST badge

Grouping:
- Rows group into collapsible minute-blocks (60 rows).
- Minute-blocks group into hour-chapters (60 blocks).

## Performance constraints
- Keep a rolling window visible (default: last 10 minutes) unless user scrolls back.
- If user scrolls away from top, pause auto-scroll and show “Return to Live.”

## Time zone switching
Switching zones reinterprets the ledger:
- instants remain the same, local timestamps change.
- show a relabeling sweep animation (or immediate redraw in reduced motion).

## DST handling (core feature)
- Spring forward: show a “gap marker” row explaining missing local times.
- Fall back: mark duplicated local timestamps as “First pass / Second pass” where possible.
- Sidebar explains today’s DST rule in plain language.

## Input
- Keyboard:
  - `J/K` scroll down/up (optional)
  - `L` return to live (top)
  - `[` `]` collapse/expand blocks
- Rotary:
  - rotate scrolls; press toggles collapse on focused block

## Accessibility
- Render ledger as semantic list/table.
- Provide density control for larger text and fewer columns.

