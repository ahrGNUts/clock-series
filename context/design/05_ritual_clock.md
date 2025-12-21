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

# Document 5 — Ritual Clock

## Concept
A clock as an **interactive ritual**: the user “conducts” the passing of seconds visually, while time remains authoritative.

## Layout
- Stage (center): 12 “chorus nodes” (hours) in a circle + 60 “beat nodes” (seconds) on an outer ring.
- Conductor panel (bottom): timezone, DST, “gesture sensitivity.”
- Overlay: minimal digital readout that appears on interaction and fades.

## Behavior
- Each second triggers a beat node “strike”.
- Each minute triggers an hour-node shimmer.
- Gestures:
  - dragging across beat nodes creates a trail that decays (visual only)
  - tapping an hour node highlights that hour in the digital overlay

## Time zone selection
- Picker styled like selecting “ensembles.”
- Switching zones triggers a subtle “retune” effect but keeps truth.

## DST motifs
- Upcoming DST: “ghost beat” warning motif.
- Fall-back repeated hour: hour nodes “echo” briefly.
- Reduced motion: motifs become static badges + clear text.

## Input
- Keyboard:
  - `T` opens time zone picker
  - `H` cycles hour highlights
  - `S` toggles “show digital overlay always”
- Rotary:
  - rotate adjusts sensitivity or cycles highlights (depending focus)

## Accessibility
- Always provide an explicit time readout mode.
- Ritual visuals are supplementary; clock remains legible via overlay and/or accessible text.


---

## Implementation Spec Appendix

### A. Stage geometry (deterministic)
Let `stageSize = min(containerWidth, containerHeight - controlsHeight)`.
Center `(cx, cy)`.

Radii:
- `rHour = 0.24 * stageSize`
- `rBeat = 0.38 * stageSize`
- `rLabel = 0.46 * stageSize` (optional)

Node sizes:
- hour node radius `0.028 * stageSize`
- beat node radius `0.012 * stageSize`

Angles:
- angle 0 at top: `θ0 = -90°`
- hour node i: `θ = θ0 + i*30°`
- beat node j: `θ = θ0 + j*6°`

Position:
- `x = cx + r*cos(θ)`
- `y = cy + r*sin(θ)`

### B. Time-to-visual mapping
- `hIndex = (hour12 % 12)` where 12 maps to 0
- `sIndex = second`
- `minuteIntensity = minute / 59`

Effects:
- On each second boundary: pulse beat node `sIndex`.
- On each minute boundary (second becomes 0): shimmer hour node `hIndex`.

### C. Animation timing (explicit)
Beat pulse:
- duration `360ms`
- `0–120ms`: scale 1.0 → 1.8
- `120–360ms`: scale 1.8 → 1.0 ease-out

Hour shimmer:
- duration `600ms`
- opacity 0 → 1 → 0 ease-in-out

Reduced motion:
- No scaling.
- Beat: ring outline for `200ms`.
- Hour shimmer: static highlight for `400ms`.

### D. Gesture trail rules
Sampling:
- sample ≤ 60 samples/sec while pointer down
- store points `{x, y, tInstant}`
- cap `maxPoints = 256` (drop oldest)

Decay:
- age `a = nowInstant - point.t`
- lifetime `L = 2.0s`
- `alpha = clamp(1 - a/L, 0..1)^2`
- width = `baseWidth * alpha`, `baseWidth = 0.010 * stageSize`

Reduced motion:
- trails disabled by default; optional toggle enables.

### E. Time zone switching choreography
On TZ change:
- “Retune” for `300ms`
- rotate entire beat ring by:
  - `Δ = (newOffsetMinutes - oldOffsetMinutes) * 0.05°`

Reduced motion: no rotation; show toast.

### F. Inputs
Keyboard:
- `T`: open picker
- `S`: toggle always-on overlay
- `ArrowLeft/Right` (stage focused): cycle hour highlight (visual only)

Rotary:
- rotate: adjust sensitivity if control focused; else cycle hour highlight
- press: toggles always-on overlay

### G. Acceptance criteria
- Beat highlight index always equals displayed second.
- Minute boundary triggers hour shimmer for local hour.
- Trails never appear in reduced motion unless explicitly enabled.

