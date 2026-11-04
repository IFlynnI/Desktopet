# firstborn, stage 4: the cleanup log

**Signed off by the owner on 2026-11-04,** after one last fix to the hatch shell pieces.

The plan approved at gate 3 promised two things:
1. automatic checks on every Frame;
2. a hand paint-over on every key, smear and hold, plus anything the checks flag.

This log is the record of both. The rig was frozen at version `199fd6e178fe`, a fingerprint of `tools/brim.py`,
`tools/brim3d.py` and `tools/pixkit.py`. Every correction below was painted on that version, and the export refuses a
correction whose base has changed since.

## How the review was done

1. All 149 drawings were rendered after their paint-over, in timing-sheet order, onto zoomed sheets, and looked at one
   by one. The long-held ones were then looked at again at 5–8×, with pixel coordinates.
2. A fault that showed in many drawings was fixed in the rig before the freeze, so it's fixed everywhere at once. One
   that showed in a single drawing was fixed by hand in the paint-over.
3. The export then runs the automatic checks on every Frame.

## Fixed in the rig before the freeze (these affect every drawing)

| Found | Fix |
|---|---|
| The mouth stamps sat on the jaw's outline, so they vanished or hung below the chin. | The mouth is now a line drawn along the snout in 3D. It follows the head as it turns, and each expression bends its corner. |
| A raised arm was hidden inside the head, so the helmet shove couldn't be seen. | The hands' paths for the brow run in front of the near cheek. |
| On the shadowed cheek, the arm was too dark to read. | Arms and hands are shaded a band lighter than the body. |
| The camera looks down, so the near foot's contact shows lower than the Anchor. The feet were losing their bottom row, and the tail in `sit` sank. | The hatchling is drawn with the ground under the near foot on the Anchor row. The rig renders 8 rows below the cell and refuses any drawing with art there. |
| Walk and run bobbed the whole body, feet included, so planted feet left the ground. | The body bobs with `torso_drop`. The feet stay put, and a lone far-foot plant is drawn on the near foot's row. |
| In `sit`, the tail and flame were placed through the torso's transform, not the tail's. | The flame uses the tail's own transform. |
| The hatch's first Frame, `egg=dict()`, was read as "no egg", because an empty dict counts as false. Brim was drawn there instead of the egg. | Tests for "is there an egg" check for `None`. The export now also checks that the hatch starts pixel for pixel like the egg. |
| The specular highlight on the shell was a large block. | The light band was narrowed. Where a block was still left, the paint-over shapes it into an arc. |
| Outline thinning broke ties by Python's random set order, so the same pose could render differently between runs. | The tie-break is sorted. Renders are now repeatable, which the paint-over fingerprints depend on. |

**At the final review** the owner saw the hatch's lower-shell pieces still floating, with one clipping Brim. The tipped
3D pieces stood up like ramps instead of lying flat, and the front one crossed Brim's belly. Now:
- the 3D crack is used only on the burst Frame, while the egg still wraps Brim;
- from then on, the pieces are hand-drawn shell fragments (`shell_tilt_*` and `shell_flat_*` in `tools/brim.py`),
  placed on the cell with their bottom on the Ground plane row, one on each side, clear of Brim's feet and tail;
- they tumble for one Frame, then lie still until they crumble to dust in the same places.

Pose fixes in the timing sheets, without changing the timing:
- the far-foot plant in walk and run,
- the feet lifting against the lean in the pounce crouch and the landings,
- the sit feet pulled in,
- the sleep hands raised,
- `poke` ending on the rest pose like the other one-shots,
- the 12 unposed in-betweens posed.

## The automatic checks (run on every Frame, every export)

| Check | Result |
|---|---|
| Stray single pixels | none |
| Gaps in the outer outline | none |
| Art on the cell's top, left or right edge | none |
| Art below the Ground plane (8 rows rendered below the cell) | none |
| A flame, puff or shard cut off by the cell edge | none |
| A grounded drawing with nothing on the bottom row (it would float) | none: every grounded drawing touches the bottom row |
| The hatch's first Frame matches the egg Form's first Frame | yes, pixel for pixel |
| The same pose gives the same pixels wherever it's drawn (118 different poses) | yes. Every repeated pose has one correction slot in `paintover/shared.txt` |
| A paint-over painted on a different base | none |
| The 11 validator rules, on the files written | pass |
| `pet-pack validate content/firstborn` (the real validator) | "Brim" is a good Pack |

## The hand paint-over

113 key, smear and hold drawings:
- 21 corrected. Poses several Animations share, such as REST and SIT, are corrected once in `paintover/shared.txt` and
  show the same wherever they appear.
- 92 reviewed with no change needed.

The 36 in-betweens get the automatic checks only, as planned.

| Animation | Drawing | Kind | What happens | Status |
|---|---|---|---|---|
| egg/idle | 1 | key | still | reviewed, no change |
| egg/idle | 2 | key | a heartbeat: the egg squashes a little | reviewed, no change |
| egg/idle | 4 | key | still | reviewed, no change |
| egg/idle | 5 | key | a tiny rock right | reviewed, no change |
| egg/idle | 7 | hold | still | reviewed, no change |
| egg/wobble | 1 | key | leans into the hand | reviewed, no change |
| egg/wobble | 2 | key | swings right | reviewed, no change |
| egg/wobble | 4 | key | swings left | reviewed, no change |
| egg/wobble | 6 | key | swings right, smaller | reviewed, no change |
| egg/wobble | 8 | hold | still, glowing | reviewed, no change |
| egg/held | 1 | key | swings right | reviewed, no change |
| egg/held | 3 | key | swings left | reviewed, no change |
| egg/falling | 1 | key | tumbling right, stretched | reviewed, no change |
| egg/falling | 3 | key | tumbling left | reviewed, no change |
| egg/landing | 1 | key | impact: squash | reviewed, no change |
| egg/landing | 2 | key | rebound: stretch | reviewed, no change |
| egg/landing | 3 | key | rocks right | reviewed, no change |
| egg/landing | 6 | hold | still: the same as idle's first Frame | reviewed, no change |
| hatchling/idle | 1 | key | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/idle | 3 | key | full breath: head and helmet up 1 px | reviewed, no change |
| hatchling/idle | 5 | key | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/idle | 6 | key | blink: half | reviewed, no change |
| hatchling/idle | 7 | key | blink: closed | reviewed, no change |
| hatchling/idle | 9 | hold | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/idle_helmet | 1 | key | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/idle_helmet | 2 | key | the helmet creeps forward | reviewed, no change |
| hatchling/idle_helmet | 3 | key | further: Brim squints | reviewed, no change |
| hatchling/idle_helmet | 4 | key | over the eyes: a comic beat | reviewed, no change |
| hatchling/idle_helmet | 5 | key | anticipation: head dips, arm drawn back | reviewed, no change |
| hatchling/idle_helmet | 6 | smear | smear: the arm swipes up and the helmet starts back | reviewed, no change |
| hatchling/idle_helmet | 7 | key | overshoot: the helmet tips back, eyes round | reviewed, no change |
| hatchling/idle_helmet | 10 | key | smug: squint and fang | corrected (7 px): dome highlight reshaped into an arc |
| hatchling/idle_helmet | 11 | hold | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/idle_puff | 1 | key | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/idle_puff | 2 | key | anticipation: a deep breath, head back, eyes shut | reviewed, no change |
| hatchling/idle_puff | 4 | key | puff: head forward, a smoke ring from the nose | reviewed, no change |
| hatchling/idle_puff | 5 | key | the puff drifts up | reviewed, no change |
| hatchling/idle_puff | 7 | hold | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/walk | 1 | key | near foot contact | reviewed, no change |
| hatchling/walk | 2 | key | down | reviewed, no change |
| hatchling/walk | 3 | key | passing | reviewed, no change |
| hatchling/walk | 4 | key | up | reviewed, no change |
| hatchling/walk | 5 | key | far foot contact | reviewed, no change |
| hatchling/walk | 6 | key | down | reviewed, no change |
| hatchling/walk | 7 | key | passing | reviewed, no change |
| hatchling/walk | 8 | key | up | reviewed, no change |
| hatchling/run | 1 | key | near foot contact | reviewed, no change |
| hatchling/run | 2 | key | push off | reviewed, no change |
| hatchling/run | 3 | key | flight | reviewed, no change |
| hatchling/run | 4 | key | far foot contact | reviewed, no change |
| hatchling/run | 5 | key | push off | reviewed, no change |
| hatchling/run | 6 | key | flight | reviewed, no change |
| hatchling/sit | 1 | key | sitting on the haunches, feet out in front, tail curled behind | corrected (5 px, shared SIT): dome highlight reshaped into a short arc |
| hatchling/sit | 3 | key | full breath | reviewed, no change |
| hatchling/sit | 5 | key | the tail tip taps | reviewed, no change |
| hatchling/sit | 6 | key | tap down | corrected (5 px, shared SIT): dome highlight reshaped into a short arc |
| hatchling/sit | 7 | hold | rest | corrected (5 px, shared SIT): dome highlight reshaped into a short arc |
| hatchling/sleep | 1 | key | dozing off sitting up: hunched forward, head drooping, helmet over the face | reviewed, no change |
| hatchling/sleep | 3 | key | full breath: the back rises | reviewed, no change |
| hatchling/sleep | 5 | hold | rest | reviewed, no change |
| hatchling/look | 1 | key | turns to look at you: helmet tipped up, eyes round | corrected (9 px): far eye removed (it sat on the snout's outline and read as a bump); dome highlight reshaped into an arc |
| hatchling/look | 2 | key | a little head tilt | corrected (4 px): far eye removed |
| hatchling/look | 3 | key | blink | reviewed, no change |
| hatchling/look | 4 | key | back from the tilt | corrected (9 px): far eye removed; dome highlight reshaped into an arc |
| hatchling/pounce | 1 | key | notices: starts to crouch | reviewed, no change |
| hatchling/pounce | 2 | key | anticipation: a deep crouch, rump up | reviewed, no change |
| hatchling/pounce | 3 | key | rump wiggle left | reviewed, no change |
| hatchling/pounce | 4 | key | rump wiggle right | reviewed, no change |
| hatchling/pounce | 5 | key | rump wiggle left | reviewed, no change |
| hatchling/pounce | 6 | smear | smear: the launch, stretched along the leap | reviewed, no change |
| hatchling/pounce | 7 | key | apex: airborne, claws out | reviewed, no change |
| hatchling/pounce | 9 | key | impact: squash, the helmet slams forward | reviewed, no change |
| hatchling/pounce | 10 | key | rebound: stretch, the helmet bounces back | reviewed, no change |
| hatchling/pounce | 12 | key | proud: “Ready!” | corrected (7 px): dome highlight reshaped into an arc |
| hatchling/pounce | 13 | hold | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/poke | 1 | key | hit: squashed sideways, eyes squeezed, the helmet jolts | reviewed, no change |
| hatchling/poke | 2 | key | recoil: stretch, the helmet pops up off the head with a “tink” | reviewed, no change |
| hatchling/poke | 4 | key | it lands crooked | reviewed, no change |
| hatchling/poke | 6 | key | settles | reviewed, no change |
| hatchling/poke | 7 | hold | glare | corrected (7 px): dome highlight reshaped into an arc |
| hatchling/poke | 8 | hold | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/petted | 1 | key | leans into the hand: helmet tipped back, eyes happy, blush; tail wags right | reviewed, no change |
| hatchling/petted | 3 | key | nuzzles: head rolls, tail wags left | reviewed, no change |
| hatchling/held | 1 | key | dangling: stretched, clutching the helmet | reviewed, no change |
| hatchling/held | 3 | key | kicks the near foot | reviewed, no change |
| hatchling/held | 5 | key | kicks the far foot | reviewed, no change |
| hatchling/falling | 1 | key | panic: wings up, the helmet lifting off | reviewed, no change |
| hatchling/falling | 3 | key | wings down | reviewed, no change |
| hatchling/landing | 1 | key | impact: a big squash, the helmet clonks over the eyes | reviewed, no change |
| hatchling/landing | 2 | key | rebound: stretch | reviewed, no change |
| hatchling/landing | 4 | key | dazed: sways under the helmet | reviewed, no change |
| hatchling/landing | 5 | key | pushes the helmet up | reviewed, no change |
| hatchling/landing | 6 | key | shakes it off | reviewed, no change |
| hatchling/landing | 7 | hold | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |
| hatchling/hatch | 1 | key | the egg, exactly as the egg Form's idle starts | reviewed, no change |
| hatchling/hatch | 2 | key | a wobble left | reviewed, no change |
| hatchling/hatch | 3 | key | a wobble right | reviewed, no change |
| hatchling/hatch | 4 | key | still: the seam glows bright (anticipation) | reviewed, no change |
| hatchling/hatch | 5 | key | a big wobble left | reviewed, no change |
| hatchling/hatch | 6 | key | a big wobble right | reviewed, no change |
| hatchling/hatch | 7 | key | squash down: the last anticipation | reviewed, no change |
| hatchling/hatch | 8 | smear | burst: the shell top shoots up, the lower shell cracks into four pieces, Brim stretches up | reviewed, no change |
| hatchling/hatch | 9 | key | the shell top flies; the pieces fall outward; Brim still stretched, eyes shut | reviewed, no change |
| hatchling/hatch | 10 | key | apex: the shell flips; the pieces lie on the ground; Brim opens its eyes | reviewed, no change |
| hatchling/hatch | 11 | key | blink | reviewed, no change |
| hatchling/hatch | 12 | key | the shell falls; Brim looks up | reviewed, no change |
| hatchling/hatch | 13 | key | it lands on Brim's head, crooked: squash | reviewed, no change |
| hatchling/hatch | 14 | key | rebound | reviewed, no change |
| hatchling/hatch | 16 | key | puzzled: eyes half shut under the crooked helmet | reviewed, no change |
| hatchling/hatch | 17 | key | pushes the helmet straight | reviewed, no change |
| hatchling/hatch | 18 | key | the pieces crumble into dust | reviewed, no change |
| hatchling/hatch | 20 | key | “Ready!” | corrected (7 px): dome highlight reshaped into an arc |
| hatchling/hatch | 21 | hold | rest | corrected (9 px, shared REST): front horn tip reduced to one pixel; dome highlight reshaped from a block into a short arc; wing-tip outline spike removed |

## How to correct a Frame later

```
py docs/art/firstborn/tools/paint.py hatchling/look 1 44,47=4 44,48=4
py docs/art/firstborn/export.py --publish
```

Drawings are numbered from 1, as in [03-timing-sheets.md](03-timing-sheets.md). Coordinates are cell pixels. `_` erases a
pixel, and `.` drops an earlier correction. See the [README](README.md).
