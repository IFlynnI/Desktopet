# firstborn: Brim, the first character Pack

Brim is a small dragon who wears the top of its eggshell as a helmet. This folder holds everything that makes the Pack:
the design decisions, the sources, the scripts and the previews. The Pack itself, which the app loads, is
[content/firstborn/](../../../content/firstborn/manifest.toml).

## The stages

| Stage | Document | Preview |
|---|---|---|
| 1. Concept | [01-concepts.md](01-concepts.md) | [previews/01-concepts.html](previews/01-concepts.html) |
| 2. Style guide | [02-style-guide.md](02-style-guide.md), with version 1 in [history/style-v1/](history/style-v1/README.md) | [previews/02-style-guide.html](previews/02-style-guide.html) |
| 3. Animation plan | [03-animation-plan.md](03-animation-plan.md), [03-timing-sheets.md](03-timing-sheets.md) | [previews/03-animatic.html](previews/03-animatic.html) |
| 4. Drawing and cleanup | [04-cleanup-log.md](04-cleanup-log.md) | [previews/05-contact-hatchling.png](previews/05-contact-hatchling.png), [previews/05-contact-egg.png](previews/05-contact-egg.png) |
| 5. The Pack | [content/firstborn/](../../../content/firstborn/manifest.toml) | [previews/05-pack.html](previews/05-pack.html), and the `previews/05-*.gif` files |

Format gaps found along the way are in [format-requests.md](format-requests.md).

## Rebuilding the Pack

Everything runs on standard-library Python 3.10 or later, with `py` on Windows. Nothing to install.

```bash
py docs/art/firstborn/export.py --publish
```

1. It builds into `docs/art/firstborn/.build/firstborn/` and runs every check.
2. It copies the Pack to `content/firstborn/` only if all of them pass.
3. The report is in `.build/report.txt`.

Without `--publish`, it only builds and checks. A full rebuild takes about 75 s the first time. Renders are cached in
`.cache/`, so later runs redraw only what changed.

To check the Pack with the app's own validator:

```bash
cargo run -p pet-content --bin pet-pack -- validate content/firstborn
```

To rebuild the previews:

```bash
py docs/art/firstborn/build_preview.py
```

`style/build_style.py` and `anim/build_animatic.py` rebuild the gate 2 and gate 3 previews.

## Changing the art

| To change | Edit | Then |
|---|---|---|
| A pose, a timing or a new Frame | `anim/hatchling.py` or `anim/egg.py`: each Animation is a timing sheet of drawings in 30 fps ticks, with a pose each | `export.py` |
| An Emote | `anim/emotes.py`: the shapes are hand-drawn pixel grids | `export.py` |
| How Brim is built: proportions, parts, shading, stamps | `tools/brim.py`, drawn by `tools/brim3d.py` | `export.py`. Any hand correction on a drawing whose base changed is reported as stale, and must be redone |
| One pixel of one Frame | `tools/paint.py` (below) | `export.py` |

### Correcting a single Frame by hand

```bash
py docs/art/firstborn/tools/paint.py hatchling/look 1 44,47=4 44,48=4
```

- `hatchling/look 1` is drawing 1 of the `look` Animation, numbered as in
  [03-timing-sheets.md](03-timing-sheets.md).
- Each `x,y=key` sets one pixel. `x,y` are cell pixels from the top-left, and `key` is a palette key from the style
  guide.
  - `_` erases a pixel.
  - `.` drops an earlier correction there.
- To see a drawing's pixels first, look at the contact sheets, or the strip in `content/firstborn/`.

Corrections live in `paintover/`, one text grid per corrected Frame, and you can also edit them by hand. Each one
records a fingerprint of the drawing it was painted on. If the rig or the pose changes underneath it, the export stops
and names the Frame, so a correction never lands on the wrong pixels.

## How a Frame is made

1. **Rig.** The pose turns a small 3D model into shapes. One ray per pixel finds the first shape it hits. This is the
   base layer.
2. **Pixel-art pass.** Flat cel bands, a 1 px outline, coloured lines inside, and hand-drawn stamps for the eyes and
   effects.
3. **Paint-over.** The hand corrections from `paintover/`.
4. **Flame.** The tail flame and effects go on top. Long drawings are split into several Frames so the flame keeps
   flickering.

## Running Brim in the app

The app loads the first Pack by folder name, and `firstborn` sorts before `placeholder`. To try Brim, point the app at
the repository's `content` folder for one run. In PowerShell, from the repository root:

```powershell
cmd /c "set DESKTOP_PET_PACKS=%CD%\content&& cargo run -p pet-app"
```

The variable is set only inside that `cmd`, so it doesn't stay in your shell, where the end-to-end tests would pick it up.
The app puts a Pet and a tray icon on your screen. Today the app plays the starting Form's default Animation, so you'll
see the egg's idle. Choosing Actions arrives with the Mind in item 4.
