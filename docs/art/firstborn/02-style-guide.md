# firstborn, stage 2: Brim's style guide

**Gate 2: passed on 2026-11-03, with version 2.**

**Changed since gate 2, while drawing the Pack:**
- The mouth is drawn along the snout in 3D, so the "Ready!" paint-over now only shapes the highlight.
- Arms are shaded a band lighter.
- The hatchling's Ground plane row sits under the near foot.
- The Body is measured from the art.

The preview is rebuilt from the final rig, so it shows these. The approved version-1 page is kept in `history/style-v1/`.

Version 1 was "a solid start" with four issues. It's kept, with the owner's notes, in
[history/style-v1/](history/style-v1/README.md).

Previews:
- [previews/02-style-guide.html](previews/02-style-guide.html) opens from disk. It answers each of the four points against
  version 1, then shows every rule below in pictures.
- [previews/02-style-guide.png](previews/02-style-guide.png) is a contact sheet: the neutral stance, "Ready!" before and
  after the paint-over, the extreme poses, and the five helmet states.

## What changed in version 2

Brim is now a **tiny 3D model drawn as pixel art.**
- Every part is a real 3D shape: ellipsoids for the head, snout, torso, thighs and feet, tubes for the tail, horns and
  arms, an eggshell for the helmet, and flat membranes for the wings.
- One fixed camera looks at all of them.
- Each pixel casts one ray, and the first shape the ray hits gives the pixel its part and its shade.
- Then the same pixel-art rules as before turn that into pixels, and the hand paint-over still goes on top.

How that answers each point:

| Your point | What fixes it |
|---|---|
| The perspective is a bit off | One camera for every part. The far horn, far eye, far wing and far foot fall where that view puts them, so they can't disagree. |
| Wings, tail, horns and legs join badly | The tail, thighs and belly are one soft surface: their shading flows across the joins with no seam line. Each wing is a membrane lying on the back, rooted at the shoulder blade, so it can't cut through the body. The horns come out of slots in the helmet. |
| The tail flame should be bigger | 7 × 10 px instead of 4 × 5, drawn by hand in four flicker Frames. |
| The helmet only shifts and clips the head | The helmet is a real shell with an inside, and it rocks about the head's centre, so it slides over the skull. Tipped back, it shows its lilac inside. Slipped forward, its brow covers the eyes. |

The cost is speed: about 0.5 s per Frame on this machine, so drawing every Frame of the Pack takes a minute or two. The
export caches nothing yet, and doesn't need to.

## Words used here

| Term | Meaning |
|---|---|
| **Ramp** | The few shades of one material, from dark to light, such as the slate blues of the scales. |
| **Hue shift** | Shadows don't just get darker; they turn towards a cooler colour, and lights towards a warmer or brighter one. It keeps small art from looking muddy. |
| **Cel bands** | Shading in a few flat steps with hard edges, like a cartoon, not a smooth blend. |
| **Coloured outline** | A line drawn in a dark shade of the part's own colour instead of black. |
| **Selective outline** | Leaving the outer outline out, or lightening it, on the lit side. Brim does **not** do this; see the line rules. |
| **Dithering** | A checkerboard of two colours to fake a third. |
| **Orthographic** | A camera with no perspective shrink: far things aren't drawn smaller. Usual for pixel art. |
| **Squash and stretch** | A body flattening on impact and lengthening when it moves fast, while keeping its volume. |
| **On twos** | Each drawing is held for two of the 30 fps ticks (66 ms) instead of one. Stage 3 uses this. |
| **Rig** | `tools/brim.py`: Brim's parts and how a pose moves them, drawn by `tools/brim3d.py`. |
| **Paint-over** | The hand-correction layer drawn on top of the rig's output for each Frame. The PRD requires this pass. |

## The camera

- **Orthographic.**
- **Brim turned 28° towards the viewer,** so the face shows and the near side of the body faces you.
- **Seen from 12° above,** so the top of the helmet shows, and the far foot stands 1–2 px higher on screen than the near
  one.
- **The near foot stands on the bottom row.** That row is the Ground plane.
- **Lit from above and slightly in front, with the light fixed to the camera.** A mirrored Frame (walking left) is lit
  the same way.

What that view implies, and the previews show:
- The near horn, eye, wing, arm and foot sit towards the back and lower.
- The far ones sit towards the snout and higher, partly hidden.
- The feet stand close together under the body.

## Palette: 21 colours for the whole Pack (unchanged)

| Ramp | Keys | Use |
|---|---|---|
| Outline | `o` #1a1326 | The outer line on every Frame. |
| Slate scales | `1` #262a52, `2` #34427a, `3` #4a64a8, `4` #6b8fd0, `5` #a6c8f0 | Body. Shadows lean violet, lights lean cyan. `5` is a rim light only. |
| Amber | `a` #7a2f2a, `b` #c2562c, `c` #ee8f3a, `d` #ffc861 | Belly plates and wing membranes. Shadows lean red, lights lean yellow. |
| Eggshell | `s` #5e5476, `t` #958aac, `u` #d8cdc6, `v` #f6efe4, `w` #ffffff | Helmet and egg. Shadows lean lilac. `s` is also the shell's inside. `w` is the one specular highlight, on the dome. |
| Horn | `h` #9c7f63, `i` #e3cfa4 | Horns and claws. |
| Glow | `r` #ff5a2e, `y` #ffcf3a, `Y` #fff4c4 | Eyes, the tail flame, the hatch seam. |
| Blush | `p` #f08aa0 | Cheeks, only when petted. |

Rules:
- **Glow belongs to three places only:** the eyes, the tail flame and the egg's seam. It's the brightest colour on Brim,
  so the eye goes to the face first.
- **Emotes use these same 21 colours.** Hearts use `r` and `p`, the Zzz uses the shell ramp, ❗ uses `y` and `r`, ❓ uses
  `u` and `4`, and 💢 uses `r`. There is room for three more colours before the 24 limit, kept for later Forms.
- The key `#` is never a colour. The grid files use it for comments and silhouettes.

## Silhouette test

Brim passes at 1×. The preview shows every pose in black at 1× and 2×. Three features must stay visible in every Frame,
because they're what makes the black shape Brim:
1. the **dome** of the helmet,
2. the **two horns** through it,
3. the **tail with its flame**.

## Proportions at the MVP size

| | Value |
|---|---|
| Hatchling cell | 64 × 80 px. Every hatchling Frame has this size. |
| Hatchling Anchor | (32, 80): bottom centre. The near foot stands on row 79. |
| Brim, standing | 57 px tall to the horn tips, 45 px wide with the tail and flame. That's inside the PRD's ≈50 × 56–110 px range. |
| Head and helmet | A little over half the height. A chibi build. |
| Body (the Form's rectangle) | `{ x = -14, y = -48, width = 28, height = 48 }`, from the helmet dome to the feet. The tail and horns stay outside it. The export measures it from the art. |
| Egg cell | 40 × 48 px, Anchor (20, 48). The egg is the same shell as the helmet, 28 × 36 px on screen. |
| Emote cell | 16 × 16 px: 32 px on screen at 2×. |

Why these numbers:
- **The cell is sized by the extreme poses.** The cell can't change per Animation, so the widest and tallest poses set it.
  Those are the pounce mid-air, the dangling held pose, the sleep curl and the hatch peak with the shell top flying. The
  preview draws all four inside it.
- **The rig refuses any art on the cell's top, left or right edge,** so a clipped horn or flame can't slip through
  unnoticed. The bottom row is where the feet stand.
- **The Body's top is the helmet dome,** because the Pack format places Emotes above the top of the Body. Sleep and sit get
  a lower Body each, as per-Action Bodies.
- **The egg is the helmet.** One 3D eggshell is used for both, cut along the same crack. The crack is a height for each
  angle around the egg: high around the face, so the eyes look out under the brow, and low at the back. On the egg it
  shows as a slanted, broken seam.

## Line rules

1. **One-pixel outline in `o`, all the way round, on every Frame.** Brim walks over any app, light or dark, so the
   outside edge can't rely on the background. That's why there's no selective outline.
2. **Lines inside the figure are coloured,** in the darkest shade of the part in front. A line is drawn only where one
   part lies over another with a real gap in depth between them, such as the arm over the chest or the chin over the
   belly.
3. **No line between parts that are one surface:** the head and snout, or the torso, belly, thighs and tail.
4. **No pinholes and no doubled lines.** The rig closes 1-pixel gaps, and an outline pixel that ends up inside the figure
   takes the part's dark colour.
5. **Curves step evenly.** Runs go 1-1-2-2-3 and not 1-3-1-2. The paint-over fixes uneven steps the rig leaves.
6. **Brim's pixels are opaque or transparent, nothing between.** Half-transparent edges would break the Hit mask and look
   grey on a dark app. The only soft shape is the drop shadow, which the renderer draws.

## Shading rules

1. **One light, fixed to the camera:** from above and slightly in front.
2. **Three or four cel bands per ramp.** The base colour covers most of a part, with a light cap on top and a dark band
   underneath.
3. **Parts of one surface share their shading.** Where a thigh meets the belly, the two shapes' normals are blended, so
   the bands run across the join instead of breaking at it.
4. **No dithering on Brim.** It shimmers as soon as a Frame changes.
5. **Textures belong to the part, not the screen.** Belly plates and speckles are placed in the part's own 3D space.
   When Brim squashes or turns, they move with it instead of crawling across it.
6. **The helmet casts a short shadow under its brow,** reaching 2 px. Behind that band the eyes glow, and that's the
   "visor" look.
7. **One specular highlight, on the dome.** The paint-over shapes it into a short arc of `w`.
8. **A lone pixel unlike all four of its neighbours, which agree, takes their shade.** This keeps band edges clean.
   Speckles are exempt.

## How the art is made

Each Frame goes through two layers:

1. **Base layer:** `tools/brim.py` turns the pose into 3D parts, and `tools/brim3d.py` draws them.
   - Rendering is deterministic: the same pose always gives the same pixels. The fingerprint check below caught a case
     where it didn't, which is now fixed.
2. **Paint-over:** hand corrections, stored as a text grid on top. `.` keeps a pixel, `_` erases it, and a palette key
   paints it.
   - Each paint-over Frame records a fingerprint of the base it was painted on.
   - If the rig changes underneath, the export refuses and names the Frame, so no correction ever lands on the wrong
     pixels.

The model sheet shows "Ready!" before and after its paint-over, which shapes the specular highlight into a short arc.
In the HTML preview you can click to flick between the two.

## The signature pose: "Ready!"

The helmet is tipped back, the visor eyes glow, and a smirk shows one fang. A fist is held forward and the chest puffed.
The tail flame is up, with a smoke puff from the nose.

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
- **The helmet**, the second actor:
  - level
  - slipped forward over the eyes
  - tipped back, showing its inside
  - crooked
  - off: the bare head, kept for rare moments
- **Blush** appears only when petted.
- **Stamps hide behind what covers them.** An eye under a slipped helmet isn't drawn.

## Motion rules that start here (stage 3 uses them)

- **Subtle motion moves whole parts in small steps.** Breathing and the helmet slipping are small moves of parts, not
  squash. Squash and stretch is kept for deliberate key Frames, such as a landing or a pounce.
- **Durations are whole 30 fps ticks:** 33, 66, 100, 133 ms and so on, each rounded down so it is never late. The app draws at 30 fps, so a 20 ms Frame might never be seen. Smears get
  at least 66 ms.

## Files

| File | What it is |
|---|---|
| `tools/pixkit.py` | Grids, PNG reading and writing, contact sheets. Standard-library Python. |
| `tools/brim3d.py` | The 3D renderer: camera, shapes, one ray per pixel, blended normals, the short helmet shadow. |
| `tools/brim.py` | Brim: the palette, the 3D parts, the pose fields, the stamps, and the line and cleanup passes. |
| `tools/paintover.py` | The paint-over layer and its fingerprint check. |
| `style/poses.py` | The poses on this guide. |
| `style/ready.paintover.txt` | The hand corrections for "Ready!". |
| `style/ready_paintover_edits.py` | Re-records those corrections on a new base, if the rig changes. Run from `docs/art/firstborn`. |
| `style/build_style.py` | Builds the previews: `py docs/art/firstborn/style/build_style.py` |
| `history/style-v1/` | Version 1, with the owner's notes. |
