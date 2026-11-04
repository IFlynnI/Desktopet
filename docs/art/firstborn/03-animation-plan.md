# firstborn, stage 3: the animation plan

**Gate 3: passed on 2026-11-03, after one revision.**

The owner's first review asked for four changes, now made:
1. **`sit`:** it partly clipped into the ground, and the legs and tail were unclear. Brim now sits on its haunches:
   - the upper body drops onto the thighs, which lie forward along the ground with a line where they cross the belly;
   - the feet stand up in front;
   - the tail lies on the ground behind and curls towards you.
   Nothing goes below the Ground plane.
2. **`sleep`:** it looked squashed. Brim now dozes off sitting up, hunched forward, head drooping and the helmet slid
   over its face. The back rises and falls with each breath.
3. **`hatch`:** Brim clipped through the lower shell from key 8. Now the lower shell cracks into four pieces on that
   same Frame. They fall away outward over the next two Frames and lie on the ground until they crumble to dust.
   At sign-off the owner asked for the pieces to sit lower, because they looked as if they floated. Each piece is now
   dropped by the height of its own lowest point, so it rests on the Ground plane as it tips.
4. **A run cycle** was added for chasing the cursor, as recommended.

Previews:
- [previews/03-animatic.html](previews/03-animatic.html) opens from disk. It plays every Animation's key poses at their
  real durations, with the timing sheet beside each one. It also has the Frame budget, a loop toggle, a mirror, a
  silhouette view, and a "walk across" view that checks the feet don't slide.
- [03-timing-sheets.md](03-timing-sheets.md) has every timing sheet as a table. It's generated from the same data as the
  animatic, in `anim/egg.py`, `anim/hatchling.py` and `anim/emotes.py`.

The style guide this builds on was approved at gate 2 ([02-style-guide.md](02-style-guide.md)).

## The Forms

`starting_form = "egg"`: the egg appears on first launch (PRG-01), and the owner hatches it.

| Form | Cell | Anchor | Body | Shadow |
|---|---|---|---|---|
| egg | 40 × 48 | (20, 48) | `{ x = -14, y = -36, width = 28, height = 36 }` | width 24, `#1a132660` |
| hatchling | 64 × 80 | (32, 80) | `{ x = -14, y = -47, width = 28, height = 47 }` | width 34, `#1a132660` |

The shadow colour is the outline colour at about 38 % opacity, so it reads as Brim's own shadow on any app.

## Actions and Situations

Actions are Pack data: the names below are what item 4's Mind will choose from. Each names the Animation it plays.

**egg**

| Action | Animation | Why |
|---|---|---|
| `idle` (default) | `idle` | The egg on first launch. Still, with a heartbeat and a tiny rock now and then. |
| `petted` | `wobble` | PRG-01: it reacts to petting with a wobble. A poke can use it too. |

| Situation | Animation |
|---|---|
| held | `held` |
| falling | `falling` |
| landing | `landing` |

**hatchling**, covering BEH-06's minimum set

| Action | Animation | Body | BEH-06 |
|---|---|---|---|
| `idle` (default) | `idle` | Form's | idle |
| `idle_helmet` | `idle_helmet` | Form's | idle variant: the helmet slips, Brim shoves it back |
| `idle_puff` | `idle_puff` | Form's | idle variant: a smoke puff |
| `walk` | `walk` | Form's | walk |
| `run` | `run` | Form's | the chase in cursor play (INT-03) |
| `follow` | `run` | Form's | follow |
| `flee` | `run` | Form's | running away after poke combos (INT-04) |
| `sit` | `sit` | `{ x = -15, y = -42, width = 30, height = 42 }` | sit |
| `sleep` | `sleep` | `{ x = -17, y = -32, width = 34, height = 32 }` | sleep |
| `look` | `look` | Form's | look-at |
| `pounce` | `pounce` | Form's | pounce |
| `poke` | `poke` | Form's | react-to-poke |
| `petted` | `petted` | Form's | being-petted |
| `hatch` | `hatch` | Form's | the hatch transformation (see below) |

| Situation | Animation | BEH-06 |
|---|---|---|
| held | `held` | being-held |
| falling | `falling` | fall |
| landing | `landing` | land |

Every Animation is played by at least one Action or Situation, as the validator requires.

## The Emote library

Pack-level, shared by both Forms, in a 16 × 16 cell (32 px on screen at 2×). They're drawn by hand in stage 4, with the
style guide's palette.

| Emote | Mode | Shows | Frames |
|---|---|---|---|
| `tired` | looping | 💤 Zs rising and fading | 4 |
| `love` | one-shot | ❤️ a heart pops, settles and floats away | 6 |
| `notice` | one-shot | ❗ pops in with a squash | 4 |
| `curious` | looping | ❓ bobbing and tilting | 4 |
| `annoyed` | looping | 💢 pulsing | 3 |

## How the principles are used

The PRD (§10.3) asks for stylised animation. Here is where each principle is used:

| Principle | Where |
|---|---|
| **Anticipation** | The pounce's crouch and rump wiggle. The deep breath before the smoke puff. The head dip before shoving the helmet up. The egg's two wobbles and its squash before it bursts. |
| **Follow-through** | The helmet settles after the shove, after a poke and after the pounce's landing. The egg's wobble gets smaller with each swing. The helmet bobs a drawing behind the walk. |
| **Squash and stretch** | Drawn into Frames by the rig, never added by code: landing, the pounce impact, the hatch burst, a poke, and the egg's heartbeat. |
| **Smears** | The pounce launch, the arm swipe in `idle_helmet`, and the hatch burst. Each lasts 2 ticks (66 ms), long enough to be seen at 30 fps. |
| **Holds** | Long holds in idle, sleep and look. The comic beat with the helmet over the eyes. The puzzled pause after the hatch. |
| **On twos and threes** | Walk is on threes (100 ms). Falling, the egg's tumble and the fast flicks are on twos (66 ms). The calm Animations use long uneven holds. |

## Timing rules

1. **Durations are whole 30 fps ticks.** The manifest's ms values come from the running total of ticks, rounded down.
   - A single Frame may read 66 or 67 ms, but the change always lands on the intended tick, and nothing drifts over a
     long Animation.
2. **The flame keeps its own clock.** An exported Frame never lasts longer than one flame step: 4 ticks normally, 2 when
   excited (petted, falling, run), 8 when asleep. The export splits long holds into Frames that differ only in the flame.
   That's why 148 drawings become 311 exported Frames.
   - The paint-over is applied before the flame, so one correction covers every flicker of a held drawing.
3. **Walk is drawn in place, at 13 px/s on screen at 1× (26 px/s at 2×). Run is drawn at 35 px/s (70 px/s at 2×).**
   - The planted foot moves back by exactly that much per drawing, so it doesn't slide as long as the simulation moves
     the Anchor at that speed.
   - The "walk across" view in the animatic checks both. Item 4 needs these numbers; format 1 has nowhere to put them.
4. **Every Animation loops, because format 1 has no play-once mode.**
   - So every Animation meant to play once (`idle_helmet`, `idle_puff`, `pounce`, `poke`, `landing`, the egg's
     `landing`, `hatch`) ends on a long hold of the pose idle starts on. When the Mind moves on, the handover can't be
     seen.
   - If it's late, the loop restarts, and the animatic's "Play as the app will" shows what that looks like.
   - The worst case is `hatch`: a restart would bring the egg back. Item 4 has to move on before its last hold ends.
5. **The hatch can't be seen switching Form.**
   - `hatch` is an Action of the hatchling Form (the workaround in [format-requests.md](format-requests.md)). Item 4
     switches to the hatchling Form, then plays it.
   - Its first Frame is the egg, drawn on the same Anchor by the same 3D shell, so it matches the egg Form's idle first
     Frame pixel for pixel. The export checks that, and fails if it ever stops being true.

## Frame budget

| | Drawings | Exported Frames |
|---|---|---|
| egg: 5 Animations | 29 | 29 |
| hatchling: 15 Animations | 119 | 282 |
| Emotes: 5 | 21 | 21 |
| **Total** | **169** | **332** |

Of the 148 rig drawings, 97 are keys, 36 in-betweens, 12 holds and 3 smears.

**What it costs:**
- Rendering all 148 drawings takes about 75 s, at about 0.5 s each.
- Renders are cached by pose and rig version, so a rebuild after a small change redraws only what changed. The animatic
  takes 35 s cold, and under a second after that.
- The PNG strips will be small: the longest is the hatch, 42 Frames × 64 px = 2,688 px wide.

**What the cleanup covers:** a careful hand paint-over of all 332 Frames pixel by pixel isn't realistic, so the plan is:
1. **Automatic checks on every Frame:**
   - stray single pixels,
   - broken outer outlines,
   - stamps cut off at the edge,
   - a grounded pose with no pixels on the bottom row,
   - the hatch's first Frame matching the egg.
2. **A hand paint-over on every key, smear and hold** (112 drawings) and on any Frame the checks flag.
3. **The in-betweens get the automatic checks only,** because they're on screen for 2–6 ticks.

## Decisions for you at this gate

1. **The timing**, as played in the animatic, including the reworked `sit`, `sleep` and `hatch` and the new `run`.
2. **The volume and the cleanup scope** above.

## For item 4 (also in the shared notes)

- The Action, Animation and Situation names above.
- Walk speed: 13 px/s at 1×. Run speed: 35 px/s at 1×.
- The pounce moves about 30 px forward during the launch and flight.
- The hatch is an Action of the hatchling Form: switch Form, then play `hatch`, then move on before its last hold ends.
- Every Animation loops. Those meant to play once end on idle's first pose, so the Mind should pick its next Action
  before they finish.

## What changed while drawing (stage 4)

None of these change the approved timing. They're recorded here so the plan matches the shipped Pack:

1. **`poke` ends on the rest pose,** with a 15-tick hold, like the other one-shots. The plan said it did, but its sheet
   ended on the glare.
2. **The shadow is 30 px wide on both Forms,** not 24 and 34. Otherwise the shadow would jump under the unchanged egg on
   the hatch's first Frame.
3. **Bodies are measured from the art by the export,** with the top at the highest row of helmet or shell:
   - egg: `y = -36`;
   - hatchling: `y = -48`;
   - `sit`: `y = -47`, `x = -15`, `width = 30`;
   - `sleep`: `y = -37`, `x = -16`, `width = 32`.

   The sit Body is nearly as tall as the standing one, because the helmet stays high when Brim sits.
4. **Markers:**
   - `footstep` on the contact Frames of walk and run;
   - `impact` on both landings and the pounce impact;
   - `hatch` on the burst.
5. **Final volume:** 149 drawings and 336 exported Frames:
   - egg 29,
   - hatchling 286,
   - Emotes 21.

   That's 4 more exported Frames than planned. The extra rest hold on `poke` adds 4, and the hatch's shell break lost one
   drawing.

The cleanup itself is recorded in [04-cleanup-log.md](04-cleanup-log.md).
