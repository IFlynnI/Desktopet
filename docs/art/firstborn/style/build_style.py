"""Builds the gate 2 style-guide previews (version 2: the 3D rig).

Run:  py docs/art/firstborn/style/build_style.py
Writes previews/02-style-guide.html (self-contained, opens from disk) and previews/02-style-guide.png.
"""

import base64
import importlib.util
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "tools"))
sys.path.insert(0, HERE)

import brim as R  # noqa: E402
import paintover as po  # noqa: E402
import pixkit as px  # noqa: E402
import poses  # noqa: E402


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


# version 1, kept in history/, drawn again for the side-by-side comparisons
V1_DIR = os.path.join(ROOT, "history", "style-v1")
sys.path.insert(0, V1_DIR)
V1 = _load("brim_rig", os.path.join(V1_DIR, "brim_rig.py"))
V1_POSES = _load("poses_v1", os.path.join(V1_DIR, "poses.py"))
sys.path.remove(V1_DIR)

PAL = R.PALETTE

# The Form's Body, relative to the Anchor: the helmet dome down to the feet, tail tip and horns left out.
BODY = dict(x=-14, y=-48, width=28, height=48)   # as the export measures it

RAMPS = [
    ("Outline", ["o"], "The outer line on every Frame, so Brim reads on any background."),
    ("Slate scales", ["1", "2", "3", "4", "5"], "Body. Shadows lean violet, lights lean cyan."),
    ("Amber", ["a", "b", "c", "d"], "Belly plates and wings. Shadows lean red, lights lean yellow."),
    ("Eggshell", ["s", "t", "u", "v", "w"], "Helmet and egg. Shadows lean lilac."),
    ("Horn", ["h", "i"], "Horns and claws."),
    ("Glow", ["r", "y", "Y"], "Eyes and the tail ember. The brightest pixels on Brim: the eye goes there first."),
    ("Blush", ["p"], "Cheeks, only when petted."),
]


def b64png(grid, palette=PAL, scale=1):
    w, h = px.grid_size(grid)
    rows = px.rgba_scale(px.grid_to_rgba(grid, palette), scale)
    return "data:image/png;base64," + base64.b64encode(px.png_bytes(w * scale, h * scale, rows)).decode()


def crop(grid, x, y, w, h):
    return [row[x:x + w] for row in grid[y:y + h]]


def proportions_png(grid, scale=6):
    """The neutral pose in its cell with the guide lines drawn on."""
    w, h = px.grid_size(grid)
    cv = px.rgba_canvas(w * scale, h * scale, (246, 246, 249, 255))
    for y in range(h):
        for x in range(w):
            if (x + y) % 2 == 0:
                px.rgba_fill(cv, x * scale, y * scale, scale, scale, (236, 236, 242, 255))
    px.rgba_blit(cv, px.rgba_scale(px.grid_to_rgba(grid, PAL), scale), 0, 0)
    ax, ay = R.ANCHOR

    def hline(y, colour):
        px.rgba_fill(cv, 0, y * scale, w * scale, 1, colour)

    def vline(x, colour):
        px.rgba_fill(cv, x * scale, 0, 1, h * scale, colour)

    def box(x, y, bw, bh, colour):
        px.rgba_fill(cv, x * scale, y * scale, bw * scale, 1, colour)
        px.rgba_fill(cv, x * scale, (y + bh) * scale - 1, bw * scale, 1, colour)
        px.rgba_fill(cv, x * scale, y * scale, 1, bh * scale, colour)
        px.rgba_fill(cv, (x + bw) * scale - 1, y * scale, 1, bh * scale, colour)

    top = px.figure_bounds(grid)[1]
    dome = min(y for y, r in enumerate(grid) if any(c in "tuvw" for c in r[18:46]))
    eye = min(y for y, r in enumerate(grid) if "Y" in r[30:60])
    belly = min(y for y, r in enumerate(grid) if any(c in "cd" for c in r[34:56]))
    hline(top, (120, 120, 255, 255))                   # top of the horns
    hline(dome, (60, 170, 90, 255))                    # top of the helmet dome
    hline(eye, (230, 120, 40, 255))                    # eye line
    hline(belly, (180, 80, 200, 255))                  # top of the belly plates
    box(ax + BODY["x"], ay + BODY["y"], BODY["width"], BODY["height"], (220, 40, 60, 255))
    px.rgba_fill(cv, (ax - 3) * scale, ay * scale - 1, 6 * scale, 1, (0, 0, 0, 255))
    vline(ax, (0, 0, 0, 90))
    box(0, 0, w, h, (60, 60, 70, 255))
    rows = [bytes(r) for r in cv]
    return "data:image/png;base64," + base64.b64encode(px.png_bytes(w * scale, h * scale, rows)).decode()


def fig(src, label, width, cls=""):
    return f'<figure class="{cls}"><img src="{src}" style="width:{width}px"><figcaption>{label}</figcaption></figure>'


def pair(a, b, region, scale, label):
    x, y, w, h = region
    return (f'<div class="pair"><h4>{label}</h4><div class="row">'
            + fig(b64png(crop(a, x, y, w, h)), "version 1", w * scale)
            + fig(b64png(crop(b, x, y, w, h)), "version 2", w * scale) + "</div></div>")


def main():
    neutral = R.draw(poses.NEUTRAL)
    ready_raw = R.draw(poses.READY)
    ready = po.apply(os.path.join(HERE, "ready.paintover.txt"), [("ready", ready_raw)])[0][1]
    expressions = [(n, R.draw(p)) for n, p in poses.EXPRESSIONS]
    helmet = [(n, R.draw(p)) for n, p in poses.HELMET]
    extremes = [(n, R.draw(p)) for n, p in poses.EXTREMES]
    eggs = [("still", R.draw_egg()), ("seam glows", R.draw_egg(glow=1)),
            ("about to hatch", R.draw_egg(glow=2)), ("wobble", R.draw_egg(rock=10, glow=1))]
    flames = [R.draw(R.pose(flame=i)) for i in range(len(R.FLAMES))]

    v1_neutral = V1.draw(V1_POSES.NEUTRAL)
    v1_helmet = [(n, V1.draw(p)) for n, p in V1_POSES.HELMET]

    sil_all = [neutral, ready] + [g for _, g in helmet] + [g for _, g in extremes] + [g for _, g in eggs]

    px.contact_sheet(os.path.join(ROOT, "previews", "02-style-guide.png"),
                     [neutral, ready_raw, ready] + [g for _, g in extremes] + [g for _, g in helmet],
                     PAL, scale=4, cols=6, cell=(64, 80))

    swatches = []
    for name, keys, note in RAMPS:
        chips = "".join(f'<span class="chip" style="background:{PAL[k]}" title="{k} {PAL[k]}"><b>{k}</b>{PAL[k]}</span>'
                        for k in keys)
        swatches.append(f'<div class="ramp"><h4>{name}</h4><div class="chips">{chips}</div><p>{note}</p></div>')

    full = (0, 22, 64, 58)
    flicker = "".join(fig(b64png(crop(g, 2, 44, 16, 20)), f"Frame {i + 1}", 16 * 6) for i, g in enumerate(flames))
    html = PAGE.format(
        n_colours=len(PAL),
        tall=px.figure_bounds(neutral)[3], wide=px.figure_bounds(neutral)[2],
        swatches="".join(swatches),
        persp=pair(v1_neutral, neutral, full, 4, "1. Perspective: one camera for every part"),
        joins="".join([
            pair(v1_neutral, neutral, (2, 46, 30, 34), 6, "2a. Where the tail grows out of the body"),
            pair(v1_neutral, neutral, (8, 46, 26, 24), 7, "2b. Where the wing roots in the back"),
            pair(v1_neutral, neutral, (12, 60, 36, 20), 6, "2c. Where the legs meet the body"),
        ]),
        flame=pair(v1_neutral, neutral, (2, 44, 16, 20), 8, "3. The tail flame")
        + f'<div class="pair"><h4>Its four flicker Frames</h4><div class="row">{flicker}</div></div>',
        helmet_v1="".join(fig(b64png(crop(g, 0, 22, 64, 58)), n, 64 * 3) for n, g in v1_helmet),
        helmet="".join(fig(b64png(crop(g, 0, 22, 64, 58)), n, 64 * 3) for n, g in helmet),
        model="".join([
            fig(b64png(crop(neutral, 0, 22, 64, 58)), "neutral stance", 64 * 4),
            fig(b64png(crop(ready_raw, 0, 22, 64, 58)), "“Ready!”: rig output", 64 * 4),
            fig(b64png(crop(ready, 0, 22, 64, 58)), "“Ready!”: after the paint-over", 64 * 4),
        ]),
        flick_a=b64png(crop(ready_raw, 12, 30, 50, 40)), flick_b=b64png(crop(ready, 12, 30, 50, 40)),
        proportions=proportions_png(neutral),
        bx=BODY["x"], by=BODY["y"], bw=BODY["width"], bh=BODY["height"],
        expressions="".join(fig(b64png(crop(g, 14, 26, 46, 36)), n, 46 * 4) for n, g in expressions),
        extremes="".join(fig(b64png(g), n, 64 * 3, "cell") for n, g in extremes),
        eggs="".join(fig(b64png(g), n, 40 * 4, "cell") for n, g in eggs),
        sil1="".join(f'<img class="sil" src="{b64png(g)}" style="width:{px.grid_size(g)[0]}px">' for g in sil_all),
        sil2="".join(f'<img class="sil" src="{b64png(g)}" style="width:{px.grid_size(g)[0] * 2}px">' for g in sil_all),
        task="".join(f'<img src="{b64png(g)}" style="width:{px.grid_size(g)[0] * 2}px">'
                     for g in [R.draw_egg(), neutral, ready, expressions[3][1]]),
    )
    with open(os.path.join(ROOT, "previews", "02-style-guide.html"), "w", encoding="utf-8") as f:
        f.write(html)
    print("wrote previews/02-style-guide.html and .png")


PAGE = """<!doctype html>
<meta charset="utf-8">
<title>firstborn: gate 2 style guide, version 2</title>
<style>
  body {{ font: 14px/1.45 "Segoe UI", system-ui, sans-serif; margin: 24px; background: #f4f4f6; color: #1b1b1f; max-width: 1250px; }}
  h1 {{ font-size: 20px; margin: 0 0 4px; }} h2 {{ font-size: 16px; margin: 30px 0 8px; }} h4 {{ margin: 0 0 6px; font-size: 13px; }}
  .hint {{ color: #555; margin: 0 0 12px; max-width: 900px; }}
  img {{ image-rendering: pixelated; display: block; }}
  .row {{ display: flex; gap: 14px; flex-wrap: wrap; align-items: flex-end; }}
  figure {{ margin: 0; background: #fff; border-radius: 8px; padding: 10px; box-shadow: 0 1px 3px #0002; }}
  figure.cell img {{ outline: 1px dashed #99a; }}
  figcaption {{ font-size: 12px; color: #555; margin-top: 6px; text-align: center; }}
  .pair {{ display: inline-block; vertical-align: top; margin: 0 22px 18px 0; }}
  .ramp {{ background: #fff; border-radius: 8px; padding: 10px 12px; box-shadow: 0 1px 3px #0002; }}
  .ramp p {{ margin: 6px 0 0; color: #555; font-size: 12px; max-width: 260px; }}
  .chips {{ display: flex; gap: 3px; }}
  .chip {{ width: 58px; height: 46px; border-radius: 5px; font: 10px/1.2 Consolas, monospace; color: #fff; padding: 3px;
          box-sizing: border-box; text-shadow: 0 0 2px #000, 0 0 2px #000; display: flex; flex-direction: column; justify-content: flex-end; }}
  .chip b {{ font-size: 13px; }}
  .strip {{ padding: 14px; border-radius: 6px; display: flex; gap: 14px; align-items: flex-end; flex-wrap: wrap; }}
  .light {{ background: #f3f3f3; }}
  .task {{ background: linear-gradient(#6b8cb4 0, #6b8cb4 calc(100% - 26px), #1c1c1c calc(100% - 26px)); padding-bottom: 26px; }}
  .sil {{ filter: brightness(0); }}
  .legend span {{ display: inline-block; margin-right: 14px; }}
  .legend i {{ display: inline-block; width: 18px; height: 3px; vertical-align: middle; margin-right: 4px; }}
  .changes {{ background: #fff8e6; border-radius: 8px; padding: 10px 16px; box-shadow: 0 1px 3px #0002; max-width: 900px; }}
</style>
<h1>firstborn: gate 2, Brim's style guide, version 2</h1>
<p class="hint">The rules are in <code>02-style-guide.md</code>. Version 1 is kept in <code>history/style-v1/</code>.</p>
<div class="changes"><b>What changed.</b> Brim is now a tiny 3D model drawn as pixel art. Every part is a real 3D shape seen
by one fixed camera: turned 28° towards you and looking down 12°. Each pixel casts one ray, and the first shape it hits
gives the pixel its part and its shade. Then the same pixel-art rules apply as before, and hand cleanup still goes on top.
This is what fixes your four points below, so each one is shown against version 1.</div>

<h2>Your four points, version 1 against version 2</h2>
<p class="hint">1. The far horn, far wing and far foot now sit towards the snout side and a little higher, partly hidden. The
near ones sit towards the back and lower. The feet stand close together under the body, not spread out like a side
view.</p>
{persp}
<p class="hint">2. The tail, the thighs and the belly are one soft surface now, so their shading flows across the joins
and there's no seam line between them. The wing is a membrane lying on the back, rooted at the shoulder blade.</p>
{joins}
<p class="hint">3. The flame is 7 × 10 px instead of 4 × 5, drawn by hand in four flicker Frames.</p>
{flame}
<h2>4. The helmet acts, and now it sits on the head</h2>
<p class="hint">The helmet is a real shell with an inside. It rocks about the head's centre, so it slides over the skull
instead of floating above it or cutting into it. Tipped back, it shows its lilac inside. Slipped forward, its brow comes
down over the eyes. The horns always pass through slots in it.</p>
<h4>version 1</h4><div class="row">{helmet_v1}</div>
<h4 style="margin-top:12px">version 2</h4><div class="row">{helmet}</div>

<h2>Palette: {n_colours} colours for the whole Pack, Emotes included (unchanged)</h2>
<div class="row">{swatches}</div>

<h2>Model sheet</h2>
<div class="row">{model}</div>
<p class="hint">Click to flick between the rig output and the cleaned Frame:
<img id="flick" src="{flick_b}" style="width:300px;display:inline-block;vertical-align:middle;cursor:pointer;outline:1px dashed #99a"></p>

<h2>Proportions in the hatchling cell (64 × 80, Anchor at 32, 80)</h2>
<div class="row"><figure><img src="{proportions}" style="width:384px"></figure>
<div class="legend" style="max-width:520px">
<p><span><i style="background:#7878ff"></i>top of the horns</span><span><i style="background:#3caa5a"></i>top of the helmet dome</span></p>
<p><span><i style="background:#e67828"></i>eyes</span><span><i style="background:#b450c8"></i>top of the belly plates</span></p>
<p><span><i style="background:#dc283c"></i>Body: x {bx}, y {by}, {bw} × {bh}, from the dome to the feet. Emotes sit above its top, so they clear the helmet.</span></p>
<p><span><i style="background:#000"></i>Anchor: where the near foot meets the Ground plane. The dotted line is the axis the art mirrors about.</span></p>
<p>Brim stands {tall} px tall to the horn tips and {wide} px wide with the tail and flame. The head and helmet are a little over half the height: a chibi build.</p>
</div></div>

<h2>Faces: the eyes do most of the acting</h2>
<div class="row">{expressions}</div>

<h2>Extreme poses: proof that the cell is big enough</h2>
<p class="hint">Dashed: the cell edge. The rig refuses any art on the cell's top, left or right edge.</p>
<div class="row">{extremes}</div>

<h2>The egg Form (cell 40 × 48, Anchor at 20, 48)</h2>
<p class="hint">The same 3D shell as the helmet. Its seam is the helmet's brow line, and it rises towards the front, so it
runs at a slant round the egg.</p>
<div class="row">{eggs}</div>

<h2>Silhouette test</h2>
<div class="strip light">{sil1}</div><p class="hint">1×, true size.</p>
<div class="strip light">{sil2}</div><p class="hint">2×, the default scale.</p>

<h2>On the desktop at 2×</h2>
<div class="strip task">{task}</div>
<script>
const a = "{flick_a}", b = "{flick_b}"; const el = document.getElementById('flick');
el.addEventListener('click', () => {{ el.src = el.src === b ? a : b; }});
</script>
"""

if __name__ == "__main__":
    main()
