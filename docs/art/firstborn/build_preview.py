"""Builds the sign-off preview from the files that ship: content/firstborn/manifest.toml and its PNG strips.

    py docs/art/firstborn/build_preview.py

Writes previews/05-pack.html (self-contained, opens from disk), a contact sheet per Form, and GIFs of the
main Animations at their real per-Frame durations if ffmpeg is on the PATH.
"""

import base64
import json
import os
import shutil
import subprocess
import sys
import tempfile
import tomllib

ROOT = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(ROOT, "..", "..", ".."))
PACK = os.path.join(REPO, "content", "firstborn")
OUT = os.path.join(ROOT, "previews")
sys.path.insert(0, os.path.join(ROOT, "tools"))

import pixkit as px  # noqa: E402


def data_uri(path):
    with open(path, "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode()


def frames_of(path, cw, ch):
    """The Frames of a strip as RGBA row lists."""
    w, h, _, rows = px.read_png(path)
    return [[r[n * cw * 4:(n + 1) * cw * 4] for r in rows] for n in range(w // cw)]


def contact(form, spec):
    """All Frames of a Form, one Animation per row, at 3x."""
    cw, ch = spec["cell"]["width"], spec["cell"]["height"]
    anims = spec["animations"]
    strips = {n: frames_of(os.path.join(PACK, a["sheet"]), cw, ch) for n, a in anims.items()}
    cols = max(len(s) for s in strips.values())
    cols = min(cols, 24)
    scale, pad = 2, 4
    rows_needed = sum((len(s) + cols - 1) // cols for s in strips.values())
    width = pad + cols * (cw * scale + pad)
    height = pad + rows_needed * (ch * scale + pad)
    cv = px.rgba_canvas(width, height, (220, 220, 228, 255))
    r = 0
    for n, s in strips.items():
        for k, fr in enumerate(s):
            x = pad + (k % cols) * (cw * scale + pad)
            y = pad + (r + k // cols) * (ch * scale + pad)
            px.rgba_blit(cv, px.rgba_scale(fr, scale), x, y)
        r += (len(s) + cols - 1) // cols
    px.write_png(os.path.join(OUT, f"05-contact-{form}.png"), width, height, [bytes(x) for x in cv])


def gif(name, path, cw, ch, durations, scale=3):
    """A GIF at the real durations, through ffmpeg's concat demuxer. Skipped if ffmpeg isn't there."""
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        return False
    tmp = tempfile.mkdtemp(prefix="firstborn-gif-")
    try:
        lines = []
        for n, fr in enumerate(frames_of(path, cw, ch)):
            canvas = px.rgba_canvas(cw, ch, (236, 236, 240, 255))
            px.rgba_blit(canvas, fr, 0, 0)
            big = px.rgba_scale([bytes(r) for r in canvas], scale)
            p = os.path.join(tmp, f"f{n:03d}.png")
            px.write_png(p, cw * scale, ch * scale, big)
            lines += [f"file '{p}'", f"duration {durations[n] / 1000:.3f}"]
        lines.append(f"file '{p}'")
        listing = os.path.join(tmp, "list.txt")
        with open(listing, "w", encoding="utf-8") as f:
            f.write("\n".join(lines).replace("\\", "/"))
        out = os.path.join(OUT, f"05-{name}.gif")
        cmd = [ffmpeg, "-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", listing,
               "-vf", "split[a][b];[a]palettegen=reserve_transparent=0[p];[b][p]paletteuse=dither=none",
               "-loop", "0", out]
        subprocess.run(cmd, check=True)
        return True
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main():
    with open(os.path.join(PACK, "manifest.toml"), "rb") as f:
        m = tomllib.load(f)
    forms = []
    for fname, spec in m["forms"].items():
        anims = []
        uses = {}
        for aname, act in spec.get("actions", {}).items():
            uses.setdefault(act["animation"], []).append(f"Action {aname}")
        for sname, aname in spec.get("situations", {}).items():
            uses.setdefault(aname, []).append(f"Situation {sname}")
        for aname, a in spec["animations"].items():
            anims.append({"name": aname, "src": data_uri(os.path.join(PACK, a["sheet"])), "ms": a["durations_ms"],
                          "markers": a.get("markers", []), "uses": uses.get(aname, [])})
        forms.append({"form": fname, "cell": [spec["cell"]["width"], spec["cell"]["height"]],
                      "anchor": [spec["anchor"]["x"], spec["anchor"]["y"]], "shadow": spec.get("shadow"),
                      "anims": anims})
        contact(fname, spec)
    em = [{"name": n, "mode": e["mode"], "src": data_uri(os.path.join(PACK, e["sheet"])), "ms": e["durations_ms"],
           "cell": [e["cell"]["width"], e["cell"]["height"]]} for n, e in m.get("emotes", {}).items()]
    data = {"name": m["name"], "starting": m["starting_form"], "forms": forms, "emotes": em}
    with open(os.path.join(ROOT, "tools", "preview-page.html"), encoding="utf-8") as f:
        page = f.read()
    with open(os.path.join(OUT, "05-pack.html"), "w", encoding="utf-8") as f:
        f.write(page.replace("/*DATA*/null", json.dumps(data)))
    made = []
    for form, anim in (("hatchling", "hatch"), ("hatchling", "idle"), ("hatchling", "walk"), ("hatchling", "run"),
                       ("hatchling", "pounce"), ("hatchling", "idle_helmet"), ("egg", "wobble")):
        spec = m["forms"][form]
        a = spec["animations"][anim]
        if gif(f"{form}-{anim}", os.path.join(PACK, a["sheet"]), spec["cell"]["width"], spec["cell"]["height"],
               a["durations_ms"]):
            made.append(anim)
    print("wrote previews/05-pack.html, the contact sheets" + (f", and GIFs of {', '.join(made)}" if made else ""))


if __name__ == "__main__":
    main()
