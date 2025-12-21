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

# Document 7 — The Clock That Refuses To Be A Clock

## Concept
A clock as a **semantic field**: time is expressed as relationships and transformations rather than digits. The primary experience is a living diagram whose geometry encodes hour/minute/second and whose topology encodes time zone + DST.

A “Truth Anchor” reveals the exact time instantly and always.

## Layout
- Full canvas diagram centered.
- Minimal HUD:
  - “Reveal Time” / Truth Anchor hint
  - timezone control icon
  - DST status dot
- Side drawer (hidden by default): explicit readout + picker + explanation.

## Primary visualization: The Temporal Grammar
Render a diagram of three nested layers:
- hour = slow “foundation” structure
- minute = mid “tension” structure
- second = fast “phase” layer

Time zone reframes the coordinate system.
DST modifies topology (twist/knot/cusp motifs).

## Required Truth Anchor
Press/hold (mouse down / touch hold / spacebar):
- overlays exact digital time at cursor (or center for keyboard)
- shows full tz + offset + DST details
Release returns to abstraction.

## Interaction
- TZ icon opens picker.
- Drag on canvas adjusts view lens only (pan/zoom), never time.
- “Decode Mode” in drawer draws guides mapping shapes to time values.

## Input
- Keyboard:
  - `Space` hold = Truth Anchor
  - `D` toggles Decode Mode
  - `Z` opens time zone picker
  - `?` opens “How to read this clock”
- Rotary:
  - rotate zooms; press toggles Truth Anchor (latched) or Decode Mode based on focus.

## Accessibility
- Provide “Explicit Mode” toggle that replaces canvas with standard readout + controls.
- Provide deterministic textual explanation of diagram state.


---

## Implementation Spec Appendix

### A. Canvas + layers
Render in order:
1) **Foundation (Hour Shape)**
2) **Tension Skin (Minute Shape)**
3) **Phase Ring (Second Shape)**
Overlays:
- Truth Anchor HUD (press/hold)
- Decode Mode guides

### B. Core parameter mapping (exact)
Inputs:
- `h = hour12` in `1..12`
- `m = minute` in `0..59`
- `s = second` in `0..59`
- `o = utcOffsetMinutes`
- `isDst` boolean
- `dstChange` (upcoming/justOccurred with `deltaMinutes` + instant)

#### 1) Hour → Foundation polygon
- `vertexCount = 3 + h` (4..15)
- `R1 = 0.28 * min(canvasW, canvasH)`
- `rotHour = (h / 12) * 30°`

Points:
- For `k = 0..vertexCount-1`:
  - `θk = -90° + rotHour + k*(360°/vertexCount)`
  - `Pk = (cx + R1*cosθk, cy + R1*sinθk)`

#### 2) Minute → Superellipse “tension skin”
Radii:
- `R2x = 0.40 * minDim`
- `R2y = 0.22 * minDim`

Exponent:
- `e = 1.2 + 2.8 * (m / 59)` (≈ 1.2..4.0)

Parametric (sample 256 points):
- For `θ in [0..2π]`:
  - `x = cx + R2x * sign(cosθ) * |cosθ|^(2/e)`
  - `y = cy + R2y * sign(sinθ) * |sinθ|^(2/e)`

Rotation:
- `rotMin = (m / 60) * 90°`
- Apply around center.

#### 3) Second → Phase ring
- `R3 = 0.46 * minDim`
- `rotSec = (s / 60) * 360°`

Render:
- 60 marks around ring; highlight mark `s`.
- Optional phase needle from center to ring at `rotSec`.

Reduced motion:
- needle snaps once per second; no smooth rotation.

### C. Time zone reframing (deterministic transform)
Apply to all layers:
- `tzRot = (o / 60) * 7.5°`
- `tzSkewX = clamp((o % 60) / 60, -1..1) * 0.10`

If `isDst = true`, add:
- `dstExtraRot = 5°`
- apply shear to minute layer only: `shearY = 0.06`

### D. DST knot and cusp behavior (mechanical)
If `dstChange.upcoming` within 24h:
- `r = remainingSeconds = dstChange.instant - nowInstant`
- `u = clamp(1 - r/(24*3600), 0..1)`
- `A = u * (0.08 * minDim)`

Anchor angle:
- `θknot = 45° + tzRot`

Draw a small loop attached to minute layer boundary at `θknot` using a 2-lobed bezier loop with outward control displacement `A`.

At transition moment:
- cusp (A max). If `justOccurred`, decay over 2 hours:
  - `A = (1 - elapsed/7200) * Amax`

Reduced motion:
- compute knot size at current instant only; no animation.

### E. Truth Anchor (exact)
Triggers:
- mouse press-hold anywhere on canvas
- touch long-press ≥ 350ms
- keyboard hold `Space`

While held:
- overlay near pointer (or centered for keyboard) with:
  - `hh:mm:ss AM/PM`
  - `Weekday, Month Day, Year`
  - `TZ name · UTC±hh:mm · DST On/Off`
  - DST change line if within 24h

Release hides overlay.

A11y:
- Write Truth Anchor content into aria-live **on press**, not continuously.

### F. Decode Mode (mandatory)
Toggle `D`.

When enabled:
- labels:
  - `V = 3 + h`
  - `e = ...`
  - `φ = s/60 * 360°`
  - `tzRot, tzSkewX`
- guide lines:
  - center → highlighted second mark
  - minute superellipse axes / bounding guides

### G. Focus order
1) TZ icon button
2) DST status dot/button
3) Truth Anchor hint chip
4) Canvas region
5) Drawer toggle
6) Drawer contents

### H. Acceptance criteria
- Same inputs yield same `vertexCount`, `e`, rotations, and transform values.
- Truth Anchor displays exact time engine outputs for selected tz.
- DST knot appears only for the defined windows and follows amplitude formula.

