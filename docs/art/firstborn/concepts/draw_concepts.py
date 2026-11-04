"""Gate 1 concept sketches: an egg and a hatchling in its signature pose for each of the three concepts.

These are rough thumbnails at roughly the MVP size, to judge silhouette and colour, not final art.
Run:  py docs/art/firstborn/concepts/draw_concepts.py
Writes concepts/<id>.txt (editable grids) and previews/01-concepts.png + previews/01-concepts.html.
"""

import base64
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "tools"))
import pixkit as px  # noqa: E402

ROOT = os.path.dirname(HERE)


def overlay(grid, pixels, key, only_on=None):
    rows = [list(r) for r in grid]
    for (x, y) in pixels:
        if 0 <= y < len(rows) and 0 <= x < len(rows[0]):
            if only_on is None or rows[y][x] in only_on:
                rows[y][x] = key
    return ["".join(r) for r in rows]


def zigzag(x0, y0, angle_deg, length, jag=2.0, segments=3):
    import math
    a = math.radians(angle_deg)
    dx, dy = math.cos(a), -math.sin(a)
    nx, ny = -dy, dx
    pts = [(x0, y0)]
    for s in range(1, segments + 1):
        t = length * s / segments
        side = jag if s % 2 else -jag
        if s == segments:
            side = 0
        pts.append((x0 + dx * t + nx * side, y0 + dy * t + ny * side))
    return pts


# ============================================================ A. Brim, the helmeted brimstone drake

BRIM = {
    "o": "#1c1526",
    "1": "#2b3a67", "2": "#3f5a99", "3": "#5f84c4", "4": "#8fb3e6",
    "a": "#a8482a", "b": "#e08a3c", "c": "#f6c065",
    "s": "#8c7ea0", "t": "#d9cdbf", "u": "#f4ede0", "w": "#ffffff",
    "h": "#a08a6e", "i": "#e8d8b0",
    "e": "#ffcc33", "E": "#fff6c2",
    "r": "#e8452c", "y": "#ffd23f",
}


def brim_hatchling():
    f = px.Figure(56, 56, "o")
    body = ["1", "2", "3", "4"]
    amber = ["a", "b", "c"]
    shell = ["s", "t", "u", "w"]
    f.add(px.Part(px.stroke([(18, 47), (10, 45), (5, 40), (5, 33)], [5, 4, 3, 2]), body[:3], shade="rim", line="1"))
    f.add(px.Part(px.polygon([(21, 38), (8, 26), (11, 33), (5, 33), (11, 39), (7, 42), (17, 43)]), amber, shade="rim", line="a"))
    f.add(px.Part(px.ellipse(21, 51, 4.5, 4.5), body, line="1", light_bias=-0.25))
    torso = px.ellipse(27, 44, 12, 10)
    f.add(px.Part(torso, body, line="1"))
    belly = px.ellipse(32, 46, 6, 7) & torso
    f.add(px.Part(belly, amber, line="a", centre=(32, 45), radii=(6, 7)))
    f.add(px.Part(px.ellipse(33, 51, 4.5, 4.5), body, line="1"))
    head = px.ellipse(31, 24, 14, 12.5)
    snout = px.ellipse(42, 28, 7.5, 5.5)
    f.add(px.Part(head, body, line="1", group="head", centre=(31, 24), radii=(14, 12.5)))
    f.add(px.Part(snout - head, body, line="1", group="head", centre=(42, 28), radii=(7.5, 5.5)))
    f.add(px.Part(px.ellipse(38, 40, 3, 3.5), body, line="1"))
    rim = [0, 1, 2, 3, 2, 1]
    dome = px.egg(30, 20, 16.5, 15, taper=0.12)
    helmet = {(x, y) for (x, y) in dome if y <= 20 + rim[x % 6]}
    f.add(px.Part(helmet, shell, line="s", centre=(28, 14), radii=(16, 13)))
    horn_back = px.stroke([(21, 9), (18, 5), (14, 3)], [2.4, 1.6, 0.7])
    horn_front = px.stroke([(35, 7), (35, 3), (32, 0)], [2.6, 1.7, 0.7])
    f.add(px.Part(horn_back, ["h", "i"], shade="rim", line="h"))
    f.add(px.Part(horn_front, ["h", "i"], shade="rim", line="h"))
    grid = f.render()
    face = (head | snout) - helmet
    shadow = {(x, y + 1) for (x, y) in helmet if (x, y + 1) in face}
    grid = overlay(grid, shadow, "1", only_on={"2", "3", "4"})
    grid = overlay(grid, {(x, y) for (x, y) in belly if y % 3 == 2}, "a", only_on={"b"})
    grid = overlay(grid, {(24, 14), (25, 14), (29, 8), (35, 15), (36, 15), (41, 12), (19, 16)}, "s", only_on={"t", "u", "w"})
    grid = px.paste(grid, ["oeEE", "oeeE", ".oee"], 36, 24)          # glowing visor eye under the rim
    grid = px.paste(grid, ["o"], 47, 26)                      # nostril
    grid = px.paste(grid, ["o....o", ".oooo."], 40, 31)       # little smirk
    grid = px.paste(grid, [".tu.", "tuuu", ".ts."], 50, 21)   # smoke puff
    grid = px.paste(grid, ["..y.", ".yy.", "yrry", "rrr.", ".r.."], 3, 27)  # tail ember
    return grid


def brim_egg():
    f = px.Figure(36, 44, "o")
    f.add(px.Part(px.egg(18, 25, 13, 18, taper=0.22), ["s", "t", "u", "w"], centre=(17, 21), radii=(13, 18)))
    grid = f.render()
    seam = [0, 1, 2, 1]
    crack = {(x, 18 + seam[x % 4]) for x in range(6, 31)}
    grid = overlay(grid, crack, "a", only_on={"s", "t", "u", "w"})
    grid = overlay(grid, {(14, 19), (15, 20), (22, 19), (23, 18)}, "e", only_on={"a"})
    grid = overlay(grid, {(11, 27), (12, 27), (22, 31), (25, 24), (16, 36), (17, 36), (9, 33)}, "2",
                   only_on={"s", "t", "u", "w"})
    return grid


# ============================================================ B. Nox, the crescent night drake

NOX = {
    "o": "#120e22",
    "1": "#1d2150", "2": "#2e3577", "3": "#4652a3", "4": "#6a7bd0",
    "a": "#6b5fa8", "b": "#9c8fd6", "c": "#c7bdf0",
    "h": "#b8953f", "i": "#f2dc8a", "j": "#fff6cf",
    "m": "#3b2a6b", "n": "#5a3f9a",
    "w": "#ffffff", "q": "#8fe3ff", "e": "#ffe27a", "k": "#120e22",
}


def nox_hatchling():
    f = px.Figure(56, 56, "o")
    body = ["1", "2", "3", "4"]
    tail = px.stroke([(18, 51), (9, 53), (3, 47), (4, 38), (9, 33), (13, 36), (11, 40)],
                     [3.6, 3.2, 2.8, 2.4, 2.0, 1.6, 1.2])
    f.add(px.Part(tail, body[:3], shade="rim", line="1"))
    horn_far = px.ellipse(33, 13, 8, 8) - px.ellipse(36, 15, 7, 7)
    horn_far = {(x, y) for (x, y) in horn_far if y < 19 and x < 38}
    f.add(px.Part(horn_far, ["h", "i"], shade="rim", line="h"))
    f.add(px.Part(px.polygon([(22, 36), (12, 30), (15, 35), (10, 38), (18, 42)]), ["m", "n"], shade="rim", line="m"))
    torso = px.ellipse(25, 45, 9.5, 10.5)
    f.add(px.Part(torso, body, line="1"))
    belly = px.ellipse(29, 46, 5, 8) & torso
    f.add(px.Part(belly, ["a", "b", "c"], line="a", centre=(29, 46), radii=(5, 8)))
    f.add(px.Part(px.stroke([(27, 38), (31, 29)], 4.5), body, line="1", group="neck"))
    head = px.ellipse(33, 24, 9.5, 8.5)
    snout = px.ellipse(41, 27, 6, 4)
    f.add(px.Part(head, body, line="1", group="neck", centre=(33, 24), radii=(9.5, 8.5)))
    f.add(px.Part(snout - head, body, line="1", group="neck", centre=(41, 27), radii=(6, 4)))
    f.add(px.Part(px.ellipse(31, 53, 4, 2.5), body, line="1"))
    f.add(px.Part(px.ellipse(21, 53, 4, 2.5), body, line="1"))
    horn_near = px.ellipse(27, 13, 9.5, 9.5) - px.ellipse(30, 15.5, 8.5, 8.5)
    horn_near = {(x, y) for (x, y) in horn_near if y < 21 and x < 33}
    f.add(px.Part(horn_near, ["h", "i", "j"], shade="rim", line="h"))
    grid = f.render()
    grid = px.paste(grid, ["oee", "eke", ".o."], 35, 22)   # cat eye with a slit pupil
    grid = px.paste(grid, ["o"], 45, 26)
    grid = px.paste(grid, ["oo"], 40, 30)
    stars = {(20, 40): "w", (24, 37): "q", (17, 45): "w", (8, 50): "q", (4, 42): "w", (11, 34): "w", (29, 30): "q"}
    for (x, y), k in stars.items():
        grid = overlay(grid, {(x, y)}, k, only_on={"1", "2", "3", "4"})
    return grid


def nox_egg():
    f = px.Figure(36, 44, "o")
    f.add(px.Part(px.egg(18, 25, 12.5, 18, taper=0.22), ["1", "2", "3", "4"], centre=(17, 20), radii=(12.5, 18)))
    grid = f.render()
    moon = px.ellipse(18, 24, 5, 5) - px.ellipse(20, 23, 4.2, 4.2)
    grid = overlay(grid, moon, "i")
    for (x, y), k in {(10, 14): "w", (25, 13): "q", (8, 30): "w", (27, 33): "w", (14, 37): "q", (23, 40): "w", (12, 22): "q"}.items():
        grid = overlay(grid, {(x, y)}, k, only_on={"1", "2", "3", "4"})
    return grid


# ============================================================ C. Volt, the storm axolotl

VOLT = {
    "o": "#1a1430",
    "1": "#3e6fa8", "2": "#63a6d8", "3": "#9ed6f0", "4": "#d6f3ff",
    "g": "#5a2a90", "G": "#a94be0", "f": "#e48af2",
    "y": "#ffe94a", "Y": "#fffbd0",
    "k": "#1a1430", "w": "#ffffff", "p": "#f08cb4",
}


def volt_hatchling():
    f = px.Figure(56, 56, "o")
    body = ["1", "2", "3", "4"]
    gills = ["g", "G", "f"]
    tips = []
    for base, ang, ln in (((24, 22), 100, 13), ((29, 20), 70, 12)):
        pts = zigzag(*base, ang, ln, jag=2)
        f.add(px.Part(px.stroke(pts, [2.0, 1.7, 1.4, 1.0]), gills[:2], shade="rim", line="g"))
        tips.append(pts[-1])
    f.add(px.Part(px.stroke([(16, 44), (8, 42), (2, 37)], [5, 3.5, 1.5]), body[:3], shade="rim", line="1"))
    f.add(px.Part(px.ellipse(15, 50, 3, 3), body, line="1", light_bias=-0.2))
    torso = px.ellipse(23, 44, 11, 7.5)
    f.add(px.Part(torso, body, line="1"))
    f.add(px.Part(px.ellipse(27, 50, 3, 3), body, line="1"))
    head = px.ellipse(32, 32, 15, 10.5)
    f.add(px.Part(head, body, line="1", centre=(32, 30), radii=(15, 10.5)))
    for base, ang, ln in (((19, 26), 140, 13), ((17, 31), 172, 13), ((19, 36), 205, 11)):
        pts = zigzag(*base, ang, ln, jag=2)
        f.add(px.Part(px.stroke(pts, [2.3, 1.9, 1.5, 1.1]), gills, shade="rim", line="g"))
        tips.append(pts[-1])
    f.add(px.Part(px.ellipse(36, 42, 3, 3), body, line="1"))
    grid = f.render()
    for (x, y) in tips:
        x, y = round(x), round(y)
        grid = px.paste(grid, [".y.", "yYy", ".y."], x - 1, y - 1)
    grid = px.paste(grid, ["kk", "kw"], 36, 27)
    grid = px.paste(grid, ["kk", "kw"], 43, 27)
    grid = px.paste(grid, ["k.........k", ".kk.....kk.", "...kkkkk..."], 34, 33)
    grid = px.paste(grid, ["pp"], 33, 31)
    grid = px.paste(grid, ["pp"], 45, 31)
    return grid


def volt_egg():
    f = px.Figure(36, 44, "1")
    f.add(px.Part(px.ellipse(18, 26, 13.5, 15.5), ["2", "3", "4", "w"], centre=(17, 22), radii=(13.5, 15.5)))
    grid = f.render()
    embryo = px.ellipse(20, 23, 4.5, 3.8) | px.stroke([(17, 26), (13, 29), (13, 33), (17, 35), (20, 33)], [3, 2.6, 2.2, 1.6, 1.0])
    grid = overlay(grid, embryo, "1", only_on={"2", "3", "4", "w"})
    grid = overlay(grid, {(22, 22)}, "o")
    grid = px.paste(grid, ["..y", ".y.", "yYy", ".y.", "y.."], 25, 13)
    grid = px.paste(grid, ["f.", ".f"], 9, 16)
    return grid


CONCEPTS = [
    ("a-brim", "Brim, the helmeted brimstone drake", BRIM, brim_egg, brim_hatchling),
    ("b-nox", "Nox, the crescent night drake", NOX, nox_egg, nox_hatchling),
    ("c-volt", "Volt, the storm axolotl", VOLT, volt_egg, volt_hatchling),
]


def b64(rows, w, h):
    return base64.b64encode(px.png_bytes(w, h, rows)).decode("ascii")


def main():
    os.makedirs(os.path.join(ROOT, "previews"), exist_ok=True)
    sheet_rows = []
    cards = []
    for cid, title, pal, egg_fn, hatch_fn in CONCEPTS:
        egg_grid, hatch_grid = egg_fn(), hatch_fn()
        px.write_grid_file(os.path.join(HERE, f"{cid}.txt"), [("egg", egg_grid), ("hatchling", hatch_grid)], pal,
                           note=f"{title}. Concept thumbnail for gate 1; generated by draw_concepts.py.")
        sheet_rows.append((egg_grid, hatch_grid, pal))
        ew, eh = px.grid_size(egg_grid)
        hw, hh = px.grid_size(hatch_grid)
        eb = px.figure_bounds(hatch_grid)
        cards.append({
            "id": cid, "title": title,
            "egg": b64(px.grid_to_rgba(egg_grid, pal), ew, eh), "ew": ew, "eh": eh,
            "hatch": b64(px.grid_to_rgba(hatch_grid, pal), hw, hh), "hw": hw, "hh": hh,
            "size": f"{eb[2]} x {eb[3]}",
            "palette": list(pal.values()),
        })

    # contact sheet PNG: per concept, egg 4x, hatchling 4x, silhouette 1x and 2x, on a light background
    scale, pad = 4, 8
    row_h = 56 * scale + pad * 2
    width = pad + 36 * scale + pad + 56 * scale + pad + 56 + pad + 56 * 2 + pad + 56 * scale + pad
    canvas = px.rgba_canvas(width, row_h * len(sheet_rows), (236, 236, 240, 255))
    for n, (egg_grid, hatch_grid, pal) in enumerate(sheet_rows):
        y0 = n * row_h + pad
        x = pad
        px.rgba_blit(canvas, px.rgba_scale(px.grid_to_rgba(egg_grid, pal), scale), x, y0 + (56 - 44) * scale)
        x += 36 * scale + pad
        px.rgba_blit(canvas, px.rgba_scale(px.grid_to_rgba(hatch_grid, pal), scale), x, y0)
        x += 56 * scale + pad
        sil = px.grid_to_rgba(px.silhouette(hatch_grid), {"#": "#000000"})
        px.rgba_blit(canvas, sil, x, y0 + 56 * scale - 56)
        x += 56 + pad
        px.rgba_blit(canvas, px.rgba_scale(sil, 2), x, y0 + 56 * scale - 112)
        x += 112 + pad
        dark = px.rgba_canvas(56 * scale, 56 * scale, (32, 32, 36, 255))
        px.rgba_blit(dark, px.rgba_scale(px.grid_to_rgba(hatch_grid, pal), scale), 0, 0)
        px.rgba_blit(canvas, [bytes(r) for r in dark], x, y0)
    out_png = os.path.join(ROOT, "previews", "01-concepts.png")
    px.write_png(out_png, width, len(canvas), [bytes(r) for r in canvas])

    import json
    with open(os.path.join(HERE, "concepts-page.html"), encoding="utf-8") as f:
        template = f.read()
    with open(os.path.join(ROOT, "previews", "01-concepts.html"), "w", encoding="utf-8") as f:
        f.write(template.replace("/*CARDS*/[]", json.dumps(cards)))
    print("wrote", out_png)


if __name__ == "__main__":
    main()
