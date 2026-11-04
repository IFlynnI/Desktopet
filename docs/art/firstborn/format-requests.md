# firstborn: proposed Pack format changes

These are things the art needs that format version 1, as agreed in
[item-3-architecture.md](../../proposals/item-3-architecture.md), can't express. They are proposals only. No code and no
proposal has been changed. The Pack works around each one, and the workaround is listed with it, so the Pack validates
against version 1 as it stands.

Status: updated at gate 3, the animation plan. Requests 1–4 were found at gate 1, and 5–8 at gate 3.

## 1. Play once, or loop, for Pet Animations

**Need:** a Pet Animation has no `mode`, unlike an Emote. Land, poke, the hatch and a yawn should play once and then
hand back to the Mind. Idle, walk and sleep loop.

**Proposal:** `mode = "looping" | "one-shot"` on `[forms.<form>.animations.<name>]`, defaulting to `looping`. The
`pet_core::animation_clock` already supports both.

**Workaround in v1:** every Animation meant to play once ends on a long hold of the pose `idle` starts on, so the hand-back to the Mind can't be seen. If the Mind is late, the loop restarts. For `hatch` that would bring the egg back, so item 4 must move on before its last hold ends.

## 2. A transformation sequence that ends in another Form

**Need:** PRD §8.4 makes transformation sequences their own kind of Animation. The hatch starts in the egg Form and
ends in the hatchling Form. v1 has nowhere to say "play this, then switch Form".

**Proposal:** a `[transformations.<name>]` table naming `from`, `to`, a sheet, its own `cell` and `anchor`, and
`durations_ms`. A sequence has its own cell because the hatch needs room for the shell top flying up. The anchors line
up the first Frame with the `from` Form and the last Frame with the `to` Form.

**Workaround in v1:** `hatch` is an Action of the hatchling Form, drawn in the hatchling's cell. Its first Frames show
the egg, placed on the same Anchor as the egg Form, so switching Form and then playing `hatch` looks seamless. That
leaves item 4 to know "play `hatch` on entering this Form", which is code knowing a name, against the "no built-in
Actions" decision.

## 3. Personality traits

**Need:** PRD §7.3 and BEH-05 put personality traits in the Pack. v1 rejects unknown fields, so there's nowhere to put
them.

**Proposal:** a `[traits]` table, added when item 4 defines what the traits are.

**Workaround in v1:** the traits are written down in the concept brief only.

## 4. Pack metadata

**Need:** credits and a short description for anyone browsing Packs. Optional, and not urgent.

**Proposal:** an optional `author` and `description`.

**Workaround in v1:** none needed. They live in `docs/art/firstborn/`.

## 5. Movement speed for walk cycles

**Need:** `walk` and `run` are drawn in place. Their planted foot moves back at 13 and 35 px/s at 1×, so the feet only
stay planted if the simulation moves the Anchor at those speeds. The number lives only in the docs.

**Proposal:** an optional `speed_px_s` on an Animation, the ground speed it was drawn for.

**Workaround in v1:** the number is in `03-animation-plan.md` and the shared notes, for item 4.

## 6. Getting into and out of a looping Action

**Need:** sitting down and getting up, or curling up to sleep and waking. Today an Action's Animation starts and stops
with a cut.

**Proposal:** an optional `intro` and `outro` Animation per Action, played once on the way in and out.

**Workaround in v1:** none. The cut is there. Each loop starts on a pose close to idle to soften it.

## 7. A Body for a Situation

**Need:** `landing` squashes Brim flat for a moment, and `held` stretches it. Only Actions can have their own Body.

**Proposal:** allow `body` on a Situation, as on an Action.

**Workaround in v1:** the Form's Body is used. It only affects bouncing off screen edges for a fraction of a second.

## 8. An overlay that animates on its own clock

**Need:** the tail flame flickers all the time, even while the body holds still. Baking it into every Frame splits each
long hold into several Frames: 148 drawings become 311 exported Frames.

**Proposal:** a per-Form `overlays` list: a small looping strip, such as the flame, with a point per Frame of the main
Animation saying where it sits. That's a bigger change, and only worth it if Packs often need it.

**Workaround in v1:** the flame is baked in by the export script, which splits long Frames.
