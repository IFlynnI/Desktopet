"""Brim's rig: draws one Frame of Brim from a pose.

Every body part is an exact shape (ellipse, egg, polygon, stroke). A pose moves, turns and squashes
the shapes, and only then are they turned into pixels, so a squashed or tilted part has clean edges
instead of a pixel-rotated, broken one. Small hand-drawn stamps (eyes, mouth, fang, effects) go on
top. The result is a grid of palette keys, facing right; the renderer mirrors it to face left.

This draws the *base layer*. Hand corrections live in the paint-over layer (see export.py).

Coordinates: shapes are written in "rig space", 64 x 72 with the feet on row 71. The hatchling cell is
64 x 80, and rig space sits in its bottom 72 rows (CELL_DY = 8), so the extra 8 rows on top are
headroom for the shell flying off in the hatch. The Anchor, where the feet meet the ground, is
(32, 80) in the cell, and the feet stand on row 79.
"""

import math

import pixkit as px

# ------------------------------------------------------------------ palette (the style guide's 22)

PALETTE = {
    "o": "#1a1326",  # outline, the darkest colour
    # slate scales: shadows lean violet, lights lean cyan
    "1": "#262a52", "2": "#34427a", "3": "#4a64a8", "4": "#6b8fd0", "5": "#a6c8f0",
    # amber belly plates and wing membranes: shadows lean red, lights lean yellow
    "a": "#7a2f2a", "b": "#c2562c", "c": "#ee8f3a", "d": "#ffc861",
    # eggshell: shadows lean lilac
    "s": "#5e5476", "t": "#958aac", "u": "#d8cdc6", "v": "#f6efe4", "w": "#ffffff",
    # horn and claw
    "h": "#9c7f63", "i": "#e3cfa4",
    # glow: eyes and the tail ember, the brightest things on the Pet
    "r": "#ff5a2e", "y": "#ffcf3a", "Y": "#fff4c4",
    # blush when petted
    "p": "#f08aa0",
}

BODY = ["2", "3", "4", "5"]
# the darkest colour of each ramp, for lines inside the figure
FAMILY_DARK = {k: "1" for k in "12345"}
FAMILY_DARK.update({k: "s" for k in "stuvw"})
FAMILY_DARK.update({k: "a" for k in "abcd"})
FAMILY_DARK.update({k: "h" for k in "hi"})
BODY_DARK = ["1", "2", "3"]
AMBER = ["b", "c", "d"]
SHELL = ["t", "u", "v", "w"]
HORN = ["h", "i"]

CELL_W, CELL_H = 64, 80
CELL_DY = 8
RIG_ANCHOR = (32, 72)
ANCHOR = (32, 80)

LIGHT = (0.0, -0.78, 0.62)


# ------------------------------------------------------------------ transforms

class Xf:
    """local -> world: scale about `pivot`, rotate about `pivot`, then move by (dx, dy)."""

    def __init__(self, pivot=(0.0, 0.0), rot=0.0, sx=1.0, sy=1.0, dx=0.0, dy=0.0):
        self.pivot, self.rot, self.sx, self.sy, self.dx, self.dy = pivot, math.radians(rot), sx, sy, dx, dy

    def fwd(self, x, y):
        px_, py_ = self.pivot
        x, y = (x - px_) * self.sx, (y - py_) * self.sy
        c, s = math.cos(self.rot), math.sin(self.rot)
        return px_ + x * c - y * s + self.dx, py_ + x * s + y * c + self.dy

    def inv(self, x, y):
        px_, py_ = self.pivot
        x, y = x - self.dx - px_, y - self.dy - py_
        c, s = math.cos(-self.rot), math.sin(-self.rot)
        x, y = x * c - y * s, x * s + y * c
        return px_ + x / self.sx, py_ + y / self.sy


class Chain:
    def __init__(self, *xfs):
        self.xfs = [x for x in xfs if x is not None]

    def fwd(self, x, y):
        for t in self.xfs:
            x, y = t.fwd(x, y)
        return x, y

    def inv(self, x, y):
        for t in reversed(self.xfs):
            x, y = t.inv(x, y)
        return x, y

    @property
    def rot(self):
        return sum(t.rot for t in self.xfs)

    def then(self, *more):
        return Chain(*self.xfs, *[m for m in more if m is not None])


# ------------------------------------------------------------------ shapes in local coordinates

class Ellipse:
    def __init__(self, cx, cy, rx, ry):
        self.cx, self.cy, self.rx, self.ry = cx, cy, rx, ry

    def bbox(self):
        return self.cx - self.rx, self.cy - self.ry, self.cx + self.rx, self.cy + self.ry

    def inside(self, x, y):
        u, v = (x - self.cx) / self.rx, (y - self.cy) / self.ry
        return u * u + v * v <= 1.0

    def normal(self, x, y):
        u, v = (x - self.cx) / self.rx, (y - self.cy) / self.ry
        return u, v, math.sqrt(max(0.0, 1 - u * u - v * v))


class Egg(Ellipse):
    """Narrower at the top by `taper`."""

    def __init__(self, cx, cy, rx, ry, taper=0.2):
        super().__init__(cx, cy, rx, ry)
        self.taper = taper

    def inside(self, x, y):
        v = (y - self.cy) / self.ry
        if abs(v) > 1:
            return False
        local = self.rx * (1 - self.taper * (-v)) if v < 0 else self.rx * (1 - self.taper * 0.15 * v)
        return abs(x - self.cx) <= local * math.sqrt(max(0.0, 1 - v * v))


class Clip:
    """`shape` limited by a test(x, y) in the same local coordinates."""

    def __init__(self, shape, test):
        self.shape, self.test = shape, test

    def bbox(self):
        return self.shape.bbox()

    def inside(self, x, y):
        return self.shape.inside(x, y) and self.test(x, y)

    def normal(self, x, y):
        return self.shape.normal(x, y)


class Poly:
    def __init__(self, pts):
        self.pts = pts

    def bbox(self):
        xs, ys = [p[0] for p in self.pts], [p[1] for p in self.pts]
        return min(xs), min(ys), max(xs), max(ys)

    def inside(self, x, y):
        return px._inside(self.pts, x, y)

    def normal(self, x, y):
        return 0.0, -0.3, 0.95


class Stroke:
    """A tapering tube through points, as a union of circles; shaded like a tube."""

    def __init__(self, pts, radii):
        self.pts = pts
        self.radii = radii if isinstance(radii, (list, tuple)) else [radii] * len(pts)

    def bbox(self):
        r = max(self.radii)
        xs, ys = [p[0] for p in self.pts], [p[1] for p in self.pts]
        return min(xs) - r, min(ys) - r, max(xs) + r, max(ys) + r

    def _nearest(self, x, y):
        best = (1e9, 0, 0, 1)
        for (x1, y1), (x2, y2), r1, r2 in zip(self.pts, self.pts[1:], self.radii, self.radii[1:]):
            vx, vy = x2 - x1, y2 - y1
            ll = vx * vx + vy * vy or 1e-9
            t = max(0.0, min(1.0, ((x - x1) * vx + (y - y1) * vy) / ll))
            qx, qy = x1 + vx * t, y1 + vy * t
            r = r1 + (r2 - r1) * t
            d = math.hypot(x - qx, y - qy)
            if d - r < best[0] - best[3]:
                best = (d, qx, qy, r)
        return best

    def inside(self, x, y):
        d, _, _, r = self._nearest(x, y)
        return d <= r

    def normal(self, x, y):
        d, qx, qy, r = self._nearest(x, y)
        u, v = (x - qx) / max(r, 0.5), (y - qy) / max(r, 0.5)
        return u, v, math.sqrt(max(0.0, 1 - u * u - v * v))


def raster(shape, xf):
    """World pixels whose centres fall inside `shape` after `xf`, with a world-space normal each."""
    x0, y0, x1, y1 = shape.bbox()
    corners = [xf.fwd(x, y) for x, y in ((x0, y0), (x1, y0), (x0, y1), (x1, y1))]
    wx0 = math.floor(min(c[0] for c in corners)) - 1
    wx1 = math.ceil(max(c[0] for c in corners)) + 1
    wy0 = math.floor(min(c[1] for c in corners)) - 1
    wy1 = math.ceil(max(c[1] for c in corners)) + 1
    rot = xf.rot
    c, s = math.cos(rot), math.sin(rot)
    out = {}
    for y in range(wy0, wy1 + 1):
        for x in range(wx0, wx1 + 1):
            lx, ly = xf.inv(x + 0.5, y + 0.5)
            if shape.inside(lx, ly):
                nx, ny, nz = shape.normal(lx, ly)
                out[(x, y)] = (nx * c - ny * s, nx * s + ny * c, nz)
    return out


# ------------------------------------------------------------------ layers

class Layer:
    def __init__(self, name, pixels, ramp, line=None, group=None, outline="o", bias=0.0, shade="sphere", bands=None):
        self.name, self.pixels, self.ramp = name, pixels, ramp
        self.line, self.group, self.outline, self.bias, self.shade = line, group, outline, bias, shade
        self.bands = bands


def _level(n, ramp_len, bias, bands):
    nx, ny, nz = n
    i = nx * LIGHT[0] + ny * LIGHT[1] + nz * LIGHT[2] + bias
    cuts = bands or {2: [0.30], 3: [0.30, 0.84], 4: [0.22, 0.62, 0.93]}[ramp_len]
    for k, cut in enumerate(cuts):
        if i < cut:
            return k
    return len(cuts)


def compose(layers, width=CELL_W, height=CELL_H):
    """Back to front. Returns (grid, owner map {pixel: layer name})."""
    owner, key = {}, {}
    for idx, layer in enumerate(layers):
        for p, n in layer.pixels.items():
            x, y = p
            # the bottom row is where the feet stand; every other edge keeps a 1-pixel margin
            if not (1 <= x < width - 1 and 1 <= y < height):
                raise ValueError(f"part '{layer.name}' reaches the cell edge at {p}; the cell is {width}x{height}")
            owner[p] = idx
            if layer.shade == "flat" or len(layer.ramp) == 1:
                key[p] = layer.ramp[min(1, len(layer.ramp) - 1)]
            else:
                key[p] = layer.ramp[_level(n, len(layer.ramp), layer.bias, layer.bands)]
    result = dict(key)
    for p, idx in owner.items():
        layer = layers[idx]
        x, y = p
        out_edge = in_edge = False
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            q = (x + dx, y + dy)
            other = owner.get(q, -1)
            if other < 0:
                out_edge = True
            elif other < idx and layer.line is not None:
                if layer.group is None or layers[other].group != layer.group:
                    in_edge = True
        if out_edge:
            result[p] = layer.outline
        elif in_edge:
            result[p] = layer.line
    grid = [[px.TRANSPARENT] * width for _ in range(height)]
    for (x, y), k in result.items():
        grid[y][x] = k
    close_gaps(grid)
    names = {p: layers[i].name for p, i in owner.items()}
    return ["".join(r) for r in grid], names


def close_gaps(grid, passes=2):
    """Fills 1-pixel transparent gaps between two outlines with outline, so the silhouette has no
    pinholes or doubled lines. A gap is a transparent pixel with opaque pixels on opposite sides."""
    h, w = len(grid), len(grid[0])
    for _ in range(passes):
        fill = []
        for y in range(1, h - 1):
            for x in range(1, w - 1):
                if grid[y][x] != px.TRANSPARENT:
                    continue
                lr = grid[y][x - 1] != px.TRANSPARENT and grid[y][x + 1] != px.TRANSPARENT
                ud = grid[y - 1][x] != px.TRANSPARENT and grid[y + 1][x] != px.TRANSPARENT
                if lr or ud:
                    fill.append((x, y))
        if not fill:
            break
        for x, y in fill:
            grid[y][x] = "o"
    # An outline pixel with no transparent neighbour sits inside the figure: a doubled line or a
    # filled gap. It takes the darkest colour of the part around it instead.
    def at(x, y):
        return grid[y][x] if 0 <= x < w and 0 <= y < h else px.TRANSPARENT

    for _ in range(4):
        thin = []
        for y in range(h):
            for x in range(w):
                if grid[y][x] != "o":
                    continue
                if px.TRANSPARENT in (at(x + 1, y), at(x - 1, y), at(x, y + 1), at(x, y - 1)):
                    continue
                ring = [at(x + dx, y + dy) for dx in (-1, 0, 1) for dy in (-1, 0, 1) if dx or dy]
                fams = [FAMILY_DARK[k] for k in ring if k in FAMILY_DARK]
                thin.append((x, y, max(set(fams), key=fams.count) if fams else "1"))
        if not thin:
            break
        for x, y, k in thin:
            grid[y][x] = k


def put(grid, rows, x, y):
    return px.paste(grid, rows, round(x), round(y))


def recolour(grid, pixels, key, only_on=None):
    rows = [list(r) for r in grid]
    for (x, y) in pixels:
        if 0 <= y < len(rows) and 0 <= x < len(rows[0]) and (only_on is None or rows[y][x] in only_on):
            rows[y][x] = key
    return ["".join(r) for r in rows]


# ------------------------------------------------------------------ the egg, shared by the egg Form and the helmet

EGG_RX, EGG_RY, EGG_TAPER = 15.5, 19.5, 0.2
SEAM_Y = -6.0                      # egg-local height of the crack line
# The crack, one value per egg-local column from x = -16 to 15: how far below SEAM_Y it runs.
# Drawn by hand to look broken, not regular. It rides high around x = 6..9, so the eye peeks out
# under the brow. The egg's seam and the helmet's brow use the same profile, so they match.
SEAM_PROFILE = [2, 3, 1, 0, 2, 4, 3, 1, 2, 3, 1, 0, 1, 3, 4, 2,
                1, 2, 3, 1, 0, 0, -1, -1, -1, 0, 1, 2, 3, 2, 1, 2]
SPECKLES = [(-7, -13), (-6, -13), (3, -15), (8, -9), (9, -9), (-11, -7), (-3, -8),
            (5, 3), (6, 3), (-8, 6), (1, 10), (2, 10), (10, 9), (-4, 14), (-12, 1)]


def seam_y(lx):
    i = max(0, min(len(SEAM_PROFILE) - 1, int(math.floor(lx)) + 16))
    return SEAM_Y + SEAM_PROFILE[i]


def egg_shape():
    return Egg(0.0, 0.0, EGG_RX, EGG_RY, EGG_TAPER)


def helmet_shape():
    return Clip(egg_shape(), lambda x, y: y <= seam_y(x) + 0.5)


def lower_shell_shape():
    return Clip(egg_shape(), lambda x, y: y > seam_y(x) + 0.5)


# ------------------------------------------------------------------ stamps

EYES = {
    # glowing visor slit under the helmet's shadow: the "cool" eye
    "visor": ["oyyYY", "oyyyY", "oyyyy", ".ooo."],
    # big round eye with a pupil: the "cute" eye
    "round": [".ooo.", "oYYyo", "oYooy", "oyooy", "oyyyo", ".ooo."],
    "half": ["ooooo", "oyyyY", ".ooo."],
    "closed": [".....", "ooooo", "....."],
    "happy": [".ooo.", "o...o"],
    "wide": [".ooo.", "oYYYo", "oYoYy", "oYYyy", "oyyyo", ".ooo."],
    "squint": [".....", "ooooo", ".yyy."],
    "angry": ["oo...", "oyyo.", ".oyyy"],
}

MOUTHS = {
    "smirk": ["o...o", ".ooo."],
    "flat": ["ooooo"],
    "fang": ["o...o", ".oww.", "...w."],
    "open": [".oooo.", "oaaaao", ".oaao."],
    "grin": ["oo...oo", ".ooooo.", "..www.."],
    "o": [".oo.", "oaao", ".oo."],
    "none": [],
}

FLAME = [
    ["..Y.", ".yy.", "yrry", "rrr.", ".r.."],
    [".Y..", ".yy.", "yrry", ".rrr", "..r."],
    ["..Y.", "..yy", ".yrr", "rrr.", ".r.."],
]

PUFF = [".uv.", "uvvv", ".ut."]


# ------------------------------------------------------------------ pose

REST = dict(
    # whole-body squash and stretch about the Anchor, and a lift (negative is up)
    sx=1.0, sy=1.0, lift=0.0, lean=0.0,
    # torso
    torso_dx=0.0, torso_dy=0.0,
    # head
    head_dx=0.0, head_dy=0.0, head_rot=0.0,
    # helmet relative to the head: slip > 0 pulls it down over the eyes
    helmet_dx=0.0, slip=0.0, helmet_rot=0.0, helmet_off=False,
    # face
    eye="visor", mouth="smirk", blush=False, nostril=True,
    # feet: (dx, dy) from rest; dy < 0 lifts the foot
    front_foot=(0.0, 0.0), back_foot=(0.0, 0.0),
    # near arm: 'rest', 'helmet' (tips the helmet up), 'reach', 'up', 'hold'
    arm="rest",
    # tail: base angle offset, curl, and which ember Frame
    tail_lift=0.0, tail_curl=0.0, flame=0,
    # wing: 0 folded, 1 half, 2 spread up, -1 spread down
    wing=0,
    puff=None,
    # free shell pieces for the hatch: (x, y, rot) in cell pixels, or None
    shell_top=None, shell_bottom=None,
    # tail wraps forward around the feet (sleep, sit)
    tail_wrap=False,
)


def pose(**kw):
    p = dict(REST)
    unknown = set(kw) - set(REST)
    if unknown:
        raise KeyError(f"unknown pose fields: {sorted(unknown)}")
    p.update(kw)
    return p


def draw(p, extras=()):
    """Returns the base-layer grid for pose `p`."""
    ax, ay = RIG_ANCHOR
    to_cell = Xf(dy=CELL_DY)
    body = Chain(Xf(pivot=(ax, ay), sx=p["sx"], sy=p["sy"], dy=p["lift"], rot=p["lean"]), to_cell)
    torso_xf = Chain(Xf(dx=p["torso_dx"], dy=p["torso_dy"]), body)
    head_pivot = (34.0, 48.0)
    head_xf = Chain(Xf(pivot=head_pivot, rot=p["head_rot"], dx=p["head_dx"], dy=p["head_dy"]), body)
    egg_origin = (32.5, 43.0)
    helmet_xf = Chain(Xf(pivot=(0, 0), rot=p["helmet_rot"], dx=egg_origin[0] + p["helmet_dx"],
                         dy=egg_origin[1] + p["slip"]), head_xf)

    layers = []

    # far wing, only when spread
    if p["wing"] != 0:
        up = {2: -1.0, 1: -0.5, -1: 0.7}[p["wing"]]
        wing_far = Poly([(26, 51), (18, 40 + up * 6), (20, 46 + up * 2), (15, 47 + up * 3), (20, 53)])
        layers.append(Layer("wing_far", raster(wing_far, torso_xf), ["a", "b", "c"], line="a", bias=-0.25))

    # tail
    tl, cu = p["tail_lift"], p["tail_curl"]
    if p["tail_wrap"]:
        tail_pts = [(22, 63), (14, 65), (17, 68), (29, 68.5), (40, 68)]
        tail = Stroke(tail_pts, [4.4, 3.6, 2.9, 2.3, 1.7])
    else:
        tail_pts = [(22, 63), (14, 62 - tl * 0.3), (9, 58 - tl * 0.7 - cu * 0.5), (8 + cu, 52 - tl - cu * 0.3)]
        tail = Stroke(tail_pts, [4.6, 3.8, 2.8, 1.9])
    if not p["tail_wrap"]:
        layers.append(Layer("tail", raster(tail, torso_xf), BODY[:3], line="2"))

    # far (back) leg
    bfx, bfy = p["back_foot"]
    back_leg = Ellipse(25 + bfx, 68.0 + bfy, 4.2, 3.8)
    layers.append(Layer("leg_back", raster(back_leg, body), BODY_DARK, line="1"))

    # torso and belly plates
    torso = Ellipse(31, 59, 11.5, 9.5)
    t_px = raster(torso, torso_xf)
    layers.append(Layer("torso", t_px, BODY, line="2"))
    belly_e = Ellipse(39.5, 61, 6.5, 8.0)
    belly = Clip(belly_e, lambda x, y: torso.inside(x, y))
    layers.append(Layer("belly", raster(belly, torso_xf), AMBER, line="b"))

    # near wing on the back
    w = p["wing"]
    if w == 0:
        wing = Poly([(27, 51), (18, 46), (20, 50), (16, 52), (22, 56)])
    elif w == 1:
        wing = Poly([(27, 51), (16, 41), (18, 47), (12, 48), (17, 53), (22, 56)])
    elif w == 2:
        wing = Poly([(27, 51), (19, 36), (20, 43), (13, 41), (16, 48), (10, 49), (22, 56)])
    else:
        wing = Poly([(27, 52), (13, 58), (18, 58), (14, 62), (21, 60), (23, 57)])
    layers.append(Layer("wing", raster(wing, torso_xf), ["b", "c", "d"], line="a"))

    # near (front) leg
    ffx, ffy = p["front_foot"]
    front_leg = Ellipse(37 + ffx, 68.0 + ffy, 4.4, 3.8)
    layers.append(Layer("leg_front", raster(front_leg, body), BODY, line="2"))

    if p["tail_wrap"]:
        layers.append(Layer("tail", raster(tail, torso_xf), BODY[:3], line="2"))

    if p["shell_bottom"]:
        bx, by, brot = p["shell_bottom"]
        bottom_px = raster(lower_shell_shape(), Chain(Xf(rot=brot, dx=bx, dy=by), to_cell))
        layers.append(Layer("shell_bottom", bottom_px, SHELL, line="t", bands=[0.30, 0.80, 0.985]))

    # the nape fills the shade under the helmet's back overhang
    nape = Ellipse(23.5, 44, 4.5, 6.0)
    layers.append(Layer("nape", raster(nape, head_xf), BODY_DARK, line="1", group="head"))

    # head and snout share a group so no line divides them
    head = Ellipse(34, 40, 13, 11.5)
    snout = Ellipse(45.5, 44, 7.0, 5.2)
    h_px = raster(head, head_xf)
    s_px = {k: v for k, v in raster(snout, head_xf).items() if k not in h_px}
    layers.append(Layer("head", h_px, BODY, line="2", group="head"))
    layers.append(Layer("snout", s_px, BODY, line="2", group="head"))

    # helmet: the top of the egg, worn on the head
    if not p["helmet_off"]:
        helm_px = raster(helmet_shape(), helmet_xf)
        layers.append(Layer("helmet", helm_px, SHELL, line="t", bands=[0.30, 0.80, 0.985]))
    else:
        helm_px = {}

    # horns poke through holes in the helmet
    horn_back = Stroke([(28, 30), (25, 25), (21, 22)], [2.5, 1.8, 0.8])
    horn_front = Stroke([(39, 29), (39, 24), (36, 20)], [2.7, 1.9, 0.8])
    hb_px, hf_px = raster(horn_back, head_xf), raster(horn_front, head_xf)
    holes = set()
    if helm_px:
        # inside the helmet only the top 2 rows of horn show; the rest is under the shell
        def emerge(horn):
            kept = {q: n for q, n in horn.items()
                    if q not in helm_px or (q[0], q[1] - 1) not in helm_px or (q[0], q[1] - 2) not in helm_px}
            for (x, y) in kept:
                if (x, y + 1) in helm_px and (x, y + 1) not in kept:
                    holes.add((x, y + 1))
            return kept
        hb_px, hf_px = emerge(hb_px), emerge(hf_px)
    layers.append(Layer("horn_back", hb_px, HORN, line="h", bias=-0.1))
    layers.append(Layer("horn_front", hf_px, HORN, line="h"))

    # near arm
    arm = p["arm"]
    if arm == "rest":
        arm_shape = Stroke([(39, 53), (41, 57)], [2.8, 2.6])
    elif arm == "helmet":
        arm_shape = Stroke([(38, 52), (44, 46), (46, 39)], [2.8, 2.4, 2.2])
    elif arm == "fist":
        arm_shape = Stroke([(39, 55), (44, 57), (46, 57)], [2.8, 2.4, 2.7])
    elif arm == "reach":
        arm_shape = Stroke([(39, 53), (45, 54)], [2.8, 2.4])
    elif arm == "up":
        arm_shape = Stroke([(38, 52), (41, 45), (42, 39)], [2.8, 2.4, 2.2])
    else:  # hold: both claws gripping the helmet rim
        arm_shape = Stroke([(38, 52), (42, 47), (44, 42)], [2.8, 2.4, 2.2])
    layers.append(Layer("arm", raster(arm_shape, torso_xf), BODY, line="2"))

    if p["shell_top"]:
        tx, ty, trot = p["shell_top"]
        top_xf = Chain(Xf(pivot=(0, -12), rot=trot), Xf(dx=tx, dy=ty), to_cell)
        layers.append(Layer("shell_top", raster(helmet_shape(), top_xf), SHELL, line="t", bands=[0.30, 0.80, 0.985]))

    for extra in extras:
        layers.append(extra(p, body, torso_xf, head_xf))

    grid, owner = compose(layers)

    # dark rims where the horns come through the shell
    grid = recolour(grid, holes, "s", only_on={"t", "u", "v", "w"})

    # the helmet casts a one-pixel shadow band on the face below its brow
    face = set(h_px) | set(s_px)
    shadow = {(x, y + 1) for (x, y) in helm_px if (x, y + 1) in face and (x, y + 1) not in helm_px}
    grid = recolour(grid, shadow, "2", only_on={"3", "4", "5"})
    # the broken edge of the shell shows its thickness as a lilac line along the brow
    brow = {(x, y) for (x, y) in helm_px if (x, y + 1) in face and (x, y + 1) not in helm_px}
    grid = recolour(grid, brow, "s", only_on={"t", "u", "v", "w"})

    # belly plates: a darker line every third row
    belly_px = [q for q, n in owner.items() if n == "belly"]
    bset = set(belly_px)
    inner = {(x, y) for (x, y) in bset if (x - 1, y) in bset and (x + 1, y) in bset}
    stripes = {(x, y) for (x, y) in inner if math.floor(torso_xf.inv(x + 0.5, y + 0.5)[1]) % 3 == 0}
    grid = recolour(grid, stripes, "b", only_on={"c"})

    # speckles on the helmet, in egg coordinates so they match the egg
    if helm_px:
        for (lx, ly) in SPECKLES:
            if ly < seam_y(lx) - 0.5:
                wx, wy = helmet_xf.fwd(lx + 0.5, ly + 0.5)
                grid = recolour(grid, {(math.floor(wx), math.floor(wy))}, "t", only_on={"u", "v"})

    # face stamps, placed in head space
    ex, ey = head_xf.fwd(39.5, 42.0)
    eye = EYES[p["eye"]]
    eye_w = len(eye[0])
    if p["eye"] in ("round", "wide"):
        grid = put(grid, eye, ex - 1, ey - 1.5)
    else:
        grid = put(grid, eye, ex, ey)
    if p["blush"]:
        bx, by = head_xf.fwd(39.0, 46.0)
        grid = recolour(grid, {(round(bx), round(by)), (round(bx) + 1, round(by))}, "p", only_on=set(BODY))
    if p["nostril"]:
        nx, ny = head_xf.fwd(50.0, 41.5)
        grid = recolour(grid, {(round(nx), round(ny))}, "o", only_on=set(BODY))
    m = MOUTHS[p["mouth"]]
    if m:
        mx, my = head_xf.fwd(43.5, 45.5)
        grid = put(grid, m, mx, my)
    # tail ember at the tip
    tipx, tipy = torso_xf.fwd(*tail_pts[-1])
    grid = put(grid, FLAME[p["flame"] % len(FLAME)], tipx - 2, tipy - 5)
    if p["puff"]:
        qx, qy = head_xf.fwd(53.0, 37.0)
        grid = put(grid, PUFF, qx + p["puff"][0], qy + p["puff"][1])
    return grid


def draw_egg(rot=0.0, dx=0.0, dy=0.0, sx=1.0, sy=1.0, seam_glow=0, cell=(40, 48), anchor=(20, 48)):
    """The egg alone. seam_glow: 0 none, 1 faint, 2 bright."""
    ax, ay = anchor
    base = Xf(pivot=(ax, ay), sx=sx, sy=sy, dx=dx, dy=dy)
    rock = Xf(pivot=(ax, ay), rot=rot)
    xf = Chain(Xf(dx=ax, dy=ay - EGG_RY), base, rock)
    shell = raster(egg_shape(), xf)
    grid, _ = compose([Layer("egg", shell, SHELL, bands=[0.30, 0.80, 0.985])], *cell)
    for (lx, ly) in SPECKLES:
        wx, wy = xf.fwd(lx + 0.5, ly + 0.5)
        grid = recolour(grid, {(math.floor(wx), math.floor(wy))}, "t", only_on={"u", "v"})
    seam = set()
    for i in range(-int(EGG_RX) - 1, int(EGG_RX) + 2):
        wx, wy = xf.fwd(i + 0.5, seam_y(i + 0.5) + 0.5)
        seam.add((math.floor(wx), math.floor(wy)))
    key = {0: "t", 1: "b", 2: "y"}[seam_glow]
    grid = recolour(grid, seam, key, only_on={"t", "u", "v", "w"})
    if seam_glow == 2:
        grid = recolour(grid, {(x, y + 1) for (x, y) in seam}, "c", only_on={"u", "v", "w"})
    return grid
