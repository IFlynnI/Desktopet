# firstborn, stage 2: Brim's style guide

**Gate 2: waiting for the owner's review.**

Previews:
- [previews/02-style-guide.html](previews/02-style-guide.html) shows every rule below in pictures. It opens from disk.
- [previews/02-style-guide.png](previews/02-style-guide.png) is a contact sheet: the neutral stance, "Ready!" before and
  after the paint-over, the extreme poses, and the helmet states.

Brim was chosen at gate 1 ([01-concepts.md](01-concepts.md)). This guide fixes how Brim looks in every Frame. Stage 3
then fixes how Brim moves.

## Words used here

| Term | Meaning |
|---|---|
| **Ramp** | The few shades of one material, from dark to light, such as the five slate blues of the scales. |
| **Hue shift** | Shadows don't just get darker; they turn towards a cooler colour, and lights towards a warmer or brighter one. It makes small art look alive instead of muddy. |
| **Cel bands** | Shading in a few flat steps with hard edges, like a cartoon, not a smooth blend. |
| **Coloured outline** | A line drawn in a dark shade of the part's own colour instead of black. |
| **Selective outline** | Leaving the outer outline out, or lightening it, on the lit side. Brim does **not** do this; see the line rules. |
| **Dithering** | A checkerboard of two colours to fake a third. |
| **Squash and stretch** | A body flattening on impact and lengthening when it moves fast, while keeping its volume. |
| **On twos** | Each drawing is held for two of the 30 fps ticks (67 ms) instead of one. Stage 3 uses this. |
| **Rig** | `tools/brim_rig.py`: Brim built from exact shapes (ellipses, an egg, tubes), posed, then turned into pixels. |
| **Paint-over** | The hand-correction layer drawn on top of the rig's output for each Frame. The PRD requires this pass. |

## Palette: 21 colours for the whole Pack

| Ramp | Keys | Use |
|---|---|---|
| Outline | `o` #1a1326 | The outer line on every Frame. |
| Slate scales | `1` #262a52, `2` #34427a, `3` #4a64a8, `4` #6b8fd0, `5` #a6c8f0 | Body. Shadows lean violet, lights lean cyan. `5` is a rim light only. |
| Amber | `a` #7a2f2a, `b` #c2562c, `c` #ee8f3a, `d` #ffc861 | Belly plates and wing membranes. Shadows lean red, lights lean yellow. |
| Eggshell | `s` #5e5476, `t` #958aac, `u` #d8cdc6, `v` #f6efe4, `w` #ffffff | Helmet and egg. Shadows lean lilac. `w` is the one specular highlight, on the dome. |
| Horn | `h` #9c7f63, `i` #e3cfa4 | Horns and claws. |
| Glow | `r` #ff5a2e, `y` #ffcf3a, `Y` #fff4c4 | Eyes, the tail ember, the hatch seam. |
| Blush | `p` #f08aa0 | Cheeks, only when petted. |

Rules:
- **Glow belongs to three places only:** the eyes, the tail ember and the egg's seam. It's the brightest colour on Brim,
  so the eye goes to the face first.
- **Emotes use these same 21 colours.** Hearts use `r` and `p`, the Zzz uses the shell ramp, ❗ uses `y` and `r`, ❓ uses
  `u` and `4`, and 💢 uses `r`. There is room for three more colours before the 24 limit, kept for later Forms.
- The key `#` is never a colour. The grid files use it for comments and silhouettes.

## Silhouette test

Brim passes at 1×. The previews show every pose in black at 1× and 2×. Three features must stay visible in every
Frame, because they are what makes the black shape Brim:
1. the **dome** of the helmet,
2. the **two horns** through it,
3. the **tail with its ember** tip.

A Frame that hides one of them, such as the sleep curl with its tail wrapped round the front, has to make up for it with
the other two.

## Proportions at the MVP size

| | Value |
|---|---|
| Hatchling cell | 64 × 80 px. Every hatchling Frame has this size. |
| Hatchling Anchor | (32, 80): bottom centre. The feet stand on row 79. |
| Brim, standing | 53 px tall to the horn tips, 46 px wide with the tail. That's inside the PRD's ≈50 × 56–110 px range. |
| Head and helmet | About 55 % of the height. A chibi build: cute and top-heavy. |
| Body (the Form's rectangle) | `{ x = -15, y = -51, width = 31, height = 51 }`, from the dome to the feet. The tail tip and horns stay outside it. |
| Egg cell | 40 × 48 px, Anchor (20, 48). The egg itself is 32 × 39 px. |
| Emote cell | 16 × 16 px: 32 px on screen at 2×. |

Why these numbers:
- **The cell is sized by the extreme poses, not the neutral one.** The cell can't change per Animation, so the widest
  and tallest poses set it. That's the pounce mid-air, the dangling held pose, the sleep curl and the hatch peak with
  the shell top flying. The preview draws all four inside the cell. The 8 rows above the horns are there for the hatch.
- **The rig refuses to draw any part within 1 px of the cell edge,** except the bottom row, where the feet stand. A clipped
  horn can't slip through unnoticed.
- **The Body's top is the helmet dome,** because the Pack format places Emotes above the top of the Body. A Body that
  stopped at the torso would put Emotes on the helmet. Sleep and sit get a lower Body each, as per-Action Bodies, so
  their Emotes sit lower too.
- **The egg is the helmet.** The helmet is exactly the top of the egg, cut along the same crack line, with the same
  speckles. You can check it in the preview: the egg's seam matches the helmet's brow.

## Line rules

1. **One-pixel outline in `o`, all the way round, on every Frame.** Brim walks over any app, light or dark, so the
   outside edge can't rely on the background. This is why there is no selective outline.
2. **Lines inside the figure are coloured,** in the darkest shade of the part in front. A line across the face is slate,
   and the helmet's brow is lilac.
3. **No line between parts that are one surface.** The head and snout are one shape, with no seam.
4. **No pinholes and no doubled lines.** The rig closes 1-pixel gaps, and an outline pixel that ends up inside the figure
   takes the part's dark colour instead.
5. **Curves step evenly.** Runs go 1-1-2-2-3 and not 1-3-1-2. The paint-over fixes any uneven steps the rig leaves.
6. **Brim's pixels are opaque or transparent, nothing between.** Half-transparent edges would break the Hit mask and
   look grey on a dark app. The only soft shape is the drop shadow, which the renderer draws.

## Shading rules

1. **One light, from above and slightly in front.** It reads the same when the renderer mirrors Brim to walk left, so a
   mirrored Frame never looks lit from the wrong side.
2. **Three or four cel bands per ramp.** The base colour covers most of a part, with a light cap on top and a dark band
   underneath.
3. **No dithering on Brim.** It shimmers as soon as a Frame changes. The egg's speckles are single pixels, not a
   dither.
4. **Textures belong to the part, not the screen.** Belly plates and speckles are placed in the part's own coordinates.
   When Brim squashes or moves, they move with it instead of crawling across it.
5. **The helmet casts a 1-pixel shadow band under its brow.** Behind that band the eyes glow, and that's the "visor"
   look.
6. **One specular highlight, on the dome.** It's a short arc of `w`, never a block.

## How the art is made

Each Frame goes through two layers:

1. **Base layer:** the rig draws the pose. It moves, turns and squashes exact shapes, and only then turns them into
   pixels, so a tilted helmet or a squashed body has clean edges instead of the broken ones a pixel rotation leaves.
2. **Paint-over:** hand corrections, stored as a text grid on top. `.` keeps a pixel, `_` erases it, and a palette key
   paints it.
   - Each paint-over Frame records a fingerprint of the base it was painted on.
   - If the rig changes underneath, the export refuses and names the Frame, so no correction ever lands on the wrong
     pixels. This happened once while building this guide: a foot moved, and the "Ready!" corrections had to be
     re-recorded.

The model sheet shows "Ready!" before and after its paint-over. In the HTML preview you can click to flick between the
two.

## The signature pose: "Ready!"

The helmet is tipped back, the visor eyes glow, and a smirk shows one fang. A fist is held forward and the chest puffed.
The tail ember is flicked high, with a smoke puff from the nose.

**Changed since the concept:** the concept had a claw tipping the helmet up. At this size an arm across the face reads
as a lower jaw, so the fist moved down in front of the chest. Lifting the helmet now happens in the idle variant
instead, where it's a moment in motion, not a held picture.

## Expressions

- **Eyes**, the main actor, each a hand-drawn stamp:
  - visor (cool, the default)
  - round (curious, cute)
  - half (bored)
  - happy (petted)
  - wide (startled)
  - angry
  - squint (smug)
  - closed (asleep)
- **Mouths:** smirk, fang, flat, open, grin, "o", none.
- **The helmet**, the second actor: level, slipped down over the eyes, tipped up, crooked, or off (the bare head: kept for
  rare moments, such as the hatch and deep contentment).
- **Blush** appears only when petted.

## Motion rules that start here (stage 3 uses them)

- **Subtle motion moves parts by whole pixels.** Breathing, the helmet slipping and an ear twitch are offsets of 1 px.
  Squash and stretch, which re-draws edges, is kept for deliberate key Frames, such as a landing or a pounce.
  Otherwise the outline would flicker on every Frame.
- **Durations are multiples of about 33 ms.** The app draws at 30 fps, so a 20 ms Frame might never be seen. Smears get
  at least 67 ms.

## Files

| File | What it is |
|---|---|
| `tools/pixkit.py` | Grids, PNG reading and writing, contact sheets. Standard-library Python. |
| `tools/brim_rig.py` | The palette, the rig and the stamps. |
| `tools/paintover.py` | The paint-over layer and its fingerprint check. |
| `style/poses.py` | The poses on this guide. |
| `style/ready.paintover.txt` | The hand corrections for "Ready!". |
| `style/build_style.py` | Builds the previews: `py docs/art/firstborn/style/build_style.py` |
