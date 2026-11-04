"""Brim, the firstborn character: palette, 3D model, poses and the pixel-art pass.

draw(pose) -> grid draws one hatchling Frame (64 x 80, Anchor at (32, 80), facing right).
draw_egg(...) -> grid draws one egg Frame (40 x 48, Anchor at (20, 48)).

How a Frame is made:
1. pose() names what every part does: where the feet are, how the helmet rocks, which eye stamp.
2. model() turns the pose into 3D parts (brim3d shapes) in model space: +x where Brim faces, +y up,
   +z towards the viewer's side, origin on the ground between the feet.
3. brim3d.render() casts one ray per pixel through the fixed camera.
4. shade() turns each hit into a palette key: flat cel bands per material, belly plates and
   speckles painted in the part's own space (so they never crawl), and the helmet's short shadow.
5. The line pass: the outer outline, coloured lines where one part lies over another, then the
   gap-closing and thinning passes and an orphan-pixel pass.
6. Hand-drawn stamps: eyes, mouth, nostril, blush, the tail flame and puffs, placed on the projected
   3D points and hidden where something covers them.
The paint-over (paintover.py) then adds the hand corrections.
"""

import math

import brim3d as B
import pixkit as px

# ------------------------------------------------------------------ palette (21 colours)

PALETTE = {
    "o": "#1a1326",
    "1": "#262a52", "2": "#34427a", "3": "#4a64a8", "4": "#6b8fd0", "5": "#a6c8f0",
    "a": "#7a2f2a", "b": "#c2562c", "c": "#ee8f3a", "d": "#ffc861",
    "s": "#5e5476", "t": "#958aac", "u": "#d8cdc6", "v": "#f6efe4", "w": "#ffffff",
    "h": "#9c7f63", "i": "#e3cfa4",
    "r": "#ff5a2e", "y": "#ffcf3a", "Y": "#fff4c4",
    "p": "#f08aa0",
}

SCALES = ["2", "3", "4", "5"]
AMBER = ["b", "c", "d"]
SHELL = ["t", "u", "v", "w"]
HORN = ["h", "i"]

FAMILY_DARK = {k: "1" for k in "12345"}
FAMILY_DARK.update({k: "s" for k in "stuvw"})
FAMILY_DARK.update({k: "a" for k in "abcd"})
FAMILY_DARK.update({k: "h" for k in "hi"})

CELL_W, CELL_H = 64, 80
ANCHOR = (32, 80)
# The camera looks down, so a point on the ground nearer the viewer shows lower on screen. The near
# foot is the nearest thing that touches the ground, so the hatchling is drawn with the ground under
# the near foot (6 px towards the viewer) on the Anchor row. Nothing may touch the ground nearer
# than that, or it would sink below the Ground plane; draw_base() refuses it.
NEAR_GROUND = 6.0
RENDER_ANCHOR = (ANCHOR[0], ANCHOR[1] - NEAR_GROUND * math.sin(math.radians(B.PITCH)))
BELOW = 8   # rows rendered below the cell, only to catch art that would sink
EGG_CELL = (40, 48)
EGG_ANCHOR = (20, 48)

# ------------------------------------------------------------------ the egg, shared by the egg Form and the helmet

EGG_R, EGG_TOP, EGG_BOTTOM, SHELL_T = 14.2, 19.0, 18.0, 1.3
SHELL_BANDS = (0.25, 0.70, 0.985)   # lilac shadow, cream, light, and a small white specular cap

# The crack, as height (in egg space) against the angle around the egg; 0 degrees is where Brim faces.
# It rides high around the face, so the eyes look out under the brow, and low at the back and far side.
# The teeth are hand-placed every 15 degrees so the break looks broken, not regular.
TEETH = [0.8, -0.9, 1.4, -0.4, 0.6, -1.3, 0.9, -0.7, 1.5, -1.0, 0.3, -0.8,
         1.1, -1.4, 0.7, -0.5, 1.2, -0.9, 0.4, -1.2, 1.0, -0.6, 1.3, -1.1]


def crack(angle):
    a = math.radians(angle)
    base = -3.5 + 12.0 * max(0.0, math.cos(a - math.radians(30))) ** 0.7
    f = ((angle + 360.0) % 360.0) / 15.0
    i = int(f)
    t = f - i
    return base + 1.8 * (TEETH[i % 24] * (1 - t) + TEETH[(i + 1) % 24] * t)


# speckles on the shell, as (angle, height) in egg space
SPECKLES = [(-150, 10), (-120, 4), (-95, 13), (-60, 8), (-35, 15), (-10, 3), (20, 11), (40, 16),
            (65, 6), (85, 14), (110, 9), (135, 2), (160, 12), (180, -4), (60, -6), (100, -10),
            (20, -9), (-40, -7), (-80, -12), (140, -11), (0, -14), (70, 0)]


def _speckle_points():
    pts = []
    for ang, y in SPECKLES:
        a = math.radians(ang)
        h = EGG_TOP if y >= 0 else EGG_BOTTOM
        rr = EGG_R * math.sqrt(max(0.0, 1 - (y / h) ** 2))
        pts.append((rr * math.cos(a), y, rr * math.sin(a)))
    return pts


SPECKLE_PTS = _speckle_points()


# ------------------------------------------------------------------ stamps (hand-drawn pixels)

EYES = {
    "visor": ["oyyYY", "oyyyy", ".ooo."],
    "round": [".oo.", "oYyo", "oYoy", "oyyo", ".oo."],
    "half": ["oooo", "oyyY", ".oo."],
    "closed": ["....", "oooo", "...."],
    "happy": [".oo.", "o..o", "...."],
    "wide": [".ooo.", "oYYyo", "oYoYy", "oyyyo", ".ooo."],
    "squint": ["....", "oooo", ".yy."],
    "angry": ["o...", ".oyy", ".oyy"],
}

MOUTHS = {
    "smirk": ["o...", ".ooo"],
    "flat": ["oooo"],
    "fang": ["o...", ".ooo", "..w."],
    "open": [".oo.", "oaao", ".oo."],
    "grin": ["o...o", ".ooo.", "..w.."],
    "o": [".o.", "oao", ".o."],
    "none": [],
}

# The tail flame: four hand-drawn flicker Frames, 7 x 10, the base sits on the tail tip.
FLAMES = [
    ["...Y...",
     "...y...",
     "..yy...",
     "..yYy..",
     ".yyYyr.",
     ".ryYyr.",
     "rryyyrr",
     ".rryrr.",
     "..rrr..",
     "...r..."],
    ["....Y..",
     "...yy..",
     "...yy..",
     "..yYy..",
     ".ryYyy.",
     ".ryYYr.",
     "rryyyrr",
     ".rryyr.",
     "..rrr..",
     "...r..."],
    [".......",
     "..Y....",
     "..yy...",
     ".yyYy..",
     ".yYYyr.",
     "ryyYyr.",
     "rryyyrr",
     ".ryyrr.",
     "..rrr..",
     "...r..."],
    ["...Y...",
     "...yy..",
     "..yYy..",
     "..yYyy.",
     ".ryYYy.",
     "rryYyrr",
     "rryyyr.",
     ".rryrr.",
     "..rrr..",
     "...r..."],
]

PUFF = [".uv.", "uvvv", ".ut."]

# The asleep flame: a low ember, two Frames.
DIM_FLAMES = [
    ["..y..",
     ".yYr.",
     "rryrr",
     ".rrr."],
    [".y...",
     ".yYr.",
     "rryyr",
     ".rrr."],
]

# Effects, stamped centred on their point.
EFFECTS = {
    "puff": PUFF,
    "puff_big": ["..uv..", ".uvvvu", "uvvvvt", ".uvvt.", "..tt.."],
    "puff_mid": [".uv.", "uvvt", ".tt."],
    "puff_small": ["uv", "tt"],
    "shard_a": [".o.", "oto", "ouo", ".o."],
    "shard_b": ["oo.", "otoo", ".ouo"],
    "spark": [".y.", "yYy", ".y."],
    # pieces of the lower shell, hand-drawn: lying on the ground with the lilac inside showing,
    # and tumbling as they fly off
    "shell_flat_l": ["o.........o.",
                     "ouo.....oouo",
                     "ouustsstuuuo",
                     ".ouuuuuuuuo.",
                     "..oooooooo.."],
    "shell_flat_r": [".o.......o",
                     "oto..oo.ouo",
                     "ouussttsuuo",
                     ".ouuuuuuuo.",
                     "..ooooooo.."],
    "shell_tilt_l": ["..oo.....",
                     ".ouuoo...",
                     "ouussuoo.",
                     "ouuuutsuo",
                     ".oouuuuuo",
                     "...ooooo."],
    "shell_tilt_r": [".....oo..",
                     "...oouuo.",
                     ".oouussuo",
                     "outsuuuuo",
                     "ouuuuuoo.",
                     ".ooooo..."],
    "spark_small": ["y"],
}


# ------------------------------------------------------------------ pose

REST = dict(
    # whole body, about the Anchor: lift (+ up), lean (+ leans back), squash and stretch
    lift=0.0, lean=0.0, sx=1.0, sy=1.0,
    # torso tilt forward (+) or back (-), degrees; drop lowers the upper body but not the legs (sitting);
    # chest swells the torso for breathing
    torso_tilt=0.0, torso_drop=0.0, chest=0.0,
    # legs: "stand", "sit" (thighs forward, feet out in front) or "tuck" (folded under, lying down)
    legs="stand",
    # head: turn towards the viewer (+), nod down (+), roll, and an offset
    head_turn=0.0, head_nod=0.0, head_roll=0.0, head_dx=0.0, head_dy=0.0,
    # helmet: rock forward over the eyes (-) or back (+), roll, or off
    rock=0.0, helmet_roll=0.0, helmet_off=False,
    # face stamps
    eye="visor", mouth="smirk", blush=False,
    # feet: (dx, dy) in model space; dy > 0 lifts the foot
    near_foot=(0.0, 0.0), far_foot=(0.0, 0.0),
    # hands: target points (model space) or None for the rest pose
    near_hand=None, far_hand=None,
    # tail: lift the tip (+ up), swing it towards the viewer (+), curl the tip
    tail_lift=0.0, tail_swing=0.0, tail_curl=0.0, flame=0,
    # or the whole tail as five points, for poses like the sleep curl; with tail_root they are in the
    # space of the legs instead of the torso, so a tail lying on the ground stays on it
    tail_path=None, tail_root=False,
    # wings: 0 folded, 1 open, 2 raised high, -1 swept down
    wings=0.0,
    # helmet lifted off the head along the horns (falling, a poke's "tink"), in pixels
    helmet_lift=0.0,
    # flame: which of the flicker Frames, and "normal" or "dim" (asleep); None draws no flame
    flame_kind="normal",
    # effects: a smoke puff from the nose as (dx, dy), and free effects as (stamp, (x, y, z) in model space);
    # fx are effects placed on the cell itself, as (stamp, centre x, bottom row): row 79 is the ground
    puff=None, effects=(), fx=(),
    # draw only the whole egg (the first Frames of the hatch): dict(rock=, sy=, sx=, glow=) or None
    egg=None,
    # free shell pieces for the hatch, (x, y, z, rot about z) in model space, or None
    shell_top=None, shell_bottom=None,
    # the lower shell broken into shards: (from angle, to angle, out, drop, tip, spin) each, in model space;
    # a drop of None rests the piece on the ground
    shards=(),
)


def pose(**kw):
    unknown = set(kw) - set(REST)
    if unknown:
        raise KeyError(f"unknown pose fields: {sorted(unknown)}")
    p = dict(REST)
    p.update(kw)
    return p


# ------------------------------------------------------------------ model

HEAD_R = (12.5, 11.5, 12.0)
SNOUT_R = (7.0, 5.2, 5.8)
SNOUT_AT = (11.5, -4.0, 0.0)       # in head space
NECK = (2.0, 22.0, 0.0)            # the head turns about this point
HEAD_AT = (0.0, 10.0, 0.0)         # head centre relative to NECK
EGG_IN_HEAD = (-1.0, -4.0, 0.0)    # the helmet's egg centre, in head space


def horn_path(side):
    z = 6.5 * side
    return [(-3.0, 8.0, z), (-4.5, 14.0, z + 1.5 * side), (-8.0, 21.0, z + 4.5 * side)]


def _arm(shoulder, hand, side):
    mid = B.lerp(shoulder, hand, 0.5)
    elbow = (mid[0] - 1.2, mid[1] - 1.5, mid[2] + 0.8 * side)
    return B.Tube([shoulder, elbow, hand], [2.6, 2.3, 2.2])


def model(p):
    """Returns the list of 3D parts for pose p, plus the transforms the stamps need."""
    root = (B.translate(0.0, p["lift"], 0.0) @ B.about((0.0, 0.0, 0.0), B.rot_z(p["lean"]))
            @ B.scale(p["sx"], p["sy"], p["sx"]))
    torso_xf = root @ B.translate(0.0, -p["torso_drop"], 0.0) @ B.about((0.0, 6.0, 0.0), B.rot_z(-p["torso_tilt"]))
    head_xf = (torso_xf @ B.translate(NECK[0] + p["head_dx"], NECK[1] + p["head_dy"], NECK[2])
               @ B.rot_y(-p["head_turn"]) @ B.rot_z(-p["head_nod"]) @ B.rot_x(p["head_roll"])
               @ B.translate(*HEAD_AT))
    parts = []

    def part(name, shape, xf, ramp, group, line, bands=(0.12, 0.60, 0.95), **kw):
        parts.append(B.Part(name, shape, xf, ramp, group, line, bands=list(bands), **kw))

    # torso: a pear, chest and belly; the tail and the thighs join it with no line
    ch = p["chest"]
    part("torso", B.Ellipsoid((10.5, 9.5 * (1 + ch), 9.0 * (1 + ch * 0.5))), torso_xf @ B.translate(-0.5, 13.0, 0.0),
         SCALES, "body", "2")
    part("belly", B.Ellipsoid((10.5, 7.5, 9.5 * (1 + ch * 0.5))), torso_xf @ B.translate(-0.5, 9.5, 0.0),
         SCALES, "body", "2")

    # tail: a tube from the lower back, tapering to the ember
    tl, ts, tc = p["tail_lift"], p["tail_swing"], p["tail_curl"]
    tail = [(-6.0, 9.0, 0.0), (-13.0, 6.5 + tl * 0.15, -1.0 + ts * 0.3), (-19.5, 7.5 + tl * 0.45, -1.5 + ts * 0.6),
            (-24.0 + tc * 0.4, 11.0 + tl * 0.8, -1.0 + ts * 0.85), (-25.5 + tc, 16.0 + tl - tc * 0.3, ts)]
    if p["tail_path"]:
        tail = [tuple(q) for q in p["tail_path"]]
    tail_xf = root if p["tail_root"] else torso_xf
    part("tail", B.Tube(tail, [5.4, 4.2, 3.2, 2.4, 1.9]), tail_xf, SCALES, "body", "2")

    # legs: a thigh merges into the body; the foot is its own piece with two claws
    for side, key in ((1, "near_foot"), (-1, "far_foot")):
        fdx, fdy = p[key]
        z = 5.5 * side
        if p["legs"] == "sit":
            # sitting on the haunches: thighs point forward along the ground, feet stand up in front
            # their own group, so a line shows where the thigh lies over the belly
            part(f"thigh_{side}", B.Ellipsoid((6.8, 4.7, 4.5)),
                 root @ B.translate(2.5 + fdx * 0.5, 4.7, z * 1.1), SCALES, f"thigh{side}", "2")
            foot_at = (9.0 + fdx, 4.0 + fdy, z)
            part(f"foot_{side}", B.Ellipsoid((3.6, 3.7, 3.6)), root @ B.translate(*foot_at), SCALES, f"foot{side}", "2")
            claw_at = (foot_at[0] + 2.9, foot_at[1] + 2.2, foot_at[2])
        elif p["legs"] == "tuck":
            # lying down: thighs folded at the sides, feet tucked under
            part(f"thigh_{side}", B.Ellipsoid((5.8, 4.4, 4.4)),
                 root @ B.translate(-2.0 + fdx * 0.5, 4.4, z * 1.1), SCALES, "body", "2")
            foot_at = (3.0 + fdx, 1.8, z * 1.15)
            part(f"foot_{side}", B.Ellipsoid((4.2, 1.8, 3.0)), root @ B.translate(*foot_at), SCALES, f"foot{side}", "2")
            claw_at = (foot_at[0] + 3.8, foot_at[1] - 0.6, foot_at[2])
        else:
            part(f"thigh_{side}", B.Ellipsoid((5.6, 6.0, 4.2)),
                 root @ B.translate(-1.5 + fdx * 0.5, 6.5 + fdy * 0.6, z), SCALES, "body", "2")
            foot_at = (1.0 + fdx, 2.4 + fdy, z + 0.5 * side)
            part(f"foot_{side}", B.Ellipsoid((5.0, 2.4, 3.4)), root @ B.translate(*foot_at), SCALES, f"foot{side}", "2")
            claw_at = (foot_at[0] + 4.6, foot_at[1] - 0.9, foot_at[2])
        for dz in (-1.4, 1.4):
            part(f"claw_{side}_{dz}", B.Ellipsoid((1.1, 0.9, 0.9)),
                 root @ B.translate(claw_at[0], claw_at[1], claw_at[2] + dz), HORN, f"foot{side}", "h", bands=(0.3,))

    # wings: a scalloped membrane and a bone along its leading edge. Each lies on the back like a
    # plate touching the torso at the shoulder blade, so it can never cut through the body; opening
    # the wings swings the plates out about their roots.
    spread = p["wings"]
    membrane = [(x * 1.25, y * 1.25) for x, y in
                [(-0.5, 1.2), (11.0, 8.0), (9.5, 4.2), (12.5, 2.0), (8.0, 0.8), (9.0, -2.4), (2.5, -2.0)]]
    torso_r, torso_c = (10.5, 9.5, 9.0), (-0.5, 13.0, 0.0)
    for side in (1, -1):
        wdir = (-0.80, 0.35, 0.50 * side)
        root_pt = B.add(torso_c, B.surface_point(torso_r, wdir))
        n = B.norm(B.sub(root_pt, torso_c))
        n = B.norm((n[0] / torso_r[0], n[1] / torso_r[1], n[2] / torso_r[2]))
        up_back = (-1.0, 0.45, 0.0)
        xa = B.norm(B.sub(up_back, B.mul(n, B.dot(up_back, n))))
        ya = (n[1] * xa[2] - n[2] * xa[1], n[2] * xa[0] - n[0] * xa[2], n[0] * xa[1] - n[1] * xa[0])
        frame = B.Affine(((xa[0], ya[0], n[0]), (xa[1], ya[1], n[1]), (xa[2], ya[2], n[2])),
                         B.add(root_pt, B.mul(n, 0.6)))
        open_ = B.rot_x(-28.0 * spread * side) @ B.rot_z(10.0 * max(0.0, spread - 1.0) - 14.0 * min(0.0, spread))
        wing_xf = torso_xf @ frame @ open_
        part(f"wing_{side}", B.Plate(membrane), wing_xf, AMBER, f"wing{side}", "a", bands=(-0.15, 0.55))
        part(f"wingbone_{side}", B.Tube([(-1.5, -0.5, 0.0), (13.75, 10.0, 0.0)], [1.7, 0.8]), wing_xf, SCALES,
             f"wing{side}", "2")

    # head and snout are one surface
    part("head", B.Ellipsoid(HEAD_R), head_xf, SCALES, "head", "2")
    part("snout", B.Ellipsoid(SNOUT_R), head_xf @ B.translate(*SNOUT_AT), SCALES, "head", "2")

    # horns, and the slots in the helmet they come through
    holes = []
    for side in (1, -1):
        path = horn_path(side)
        part(f"horn_{side}", B.Tube(path, [3.0, 2.1, 1.05]), head_xf, HORN, "horn", "h", bands=(0.45,))
        holes.append((path[0], B.norm(B.sub(path[1], path[0])), 3.0))

    if not p["helmet_off"]:
        # rock about an axis turned towards the near side, so slipping forward covers the eyes the camera sees
        local = (B.translate(0.0, p["helmet_lift"], 0.0)
                 @ B.rot_y(-35.0) @ B.rot_z(p["rock"]) @ B.rot_y(35.0) @ B.rot_x(p["helmet_roll"])
                 @ B.translate(*EGG_IN_HEAD))
        shell = B.Shell(EGG_R, EGG_TOP, EGG_BOTTOM, SHELL_T, crack, keep_top=True, holes=holes, holes_xf=local)
        part("helmet", shell, head_xf @ local, SHELL, "helmet", "t", bands=SHELL_BANDS)

    # arms: shoulder to hand, with a little round hand
    for side, key, rest in ((1, "near_hand", (8.5, 10.5, 8.5)), (-1, "far_hand", (8.0, 11.0, -7.0))):
        shoulder = (4.0, 16.5, 6.5 * side)
        hand = p[key] or rest
        # arms are shaded a band lighter than the body, so a raised arm reads against a shadowed cheek
        part(f"arm_{side}", _arm(shoulder, hand, side), torso_xf, SCALES, f"arm{side}", "2", bands=(-0.25, 0.35, 0.85))
        part(f"hand_{side}", B.Ellipsoid((2.5, 2.4, 2.4)), torso_xf @ B.translate(*hand), SCALES, f"arm{side}", "2",
             bands=(-0.25, 0.30, 0.80))

    # free shell pieces for the hatch
    if p["shell_bottom"]:
        x, y, z, rot = p["shell_bottom"]
        shell = B.Shell(EGG_R, EGG_TOP, EGG_BOTTOM, SHELL_T, crack, keep_top=False)
        part("shell_bottom", shell, B.translate(x, y, z) @ B.rot_z(rot), SHELL, "shellb", "t", bands=(0.25, 0.70, 0.97))
    for n, (a0, a1, out, drop, tip, spin) in enumerate(p["shards"]):
        # a piece of the lower shell: pushed out from the middle, dropped, tipped over its bottom
        # outer edge (as if falling outward), and spun about the vertical
        mid = math.radians((a0 + (((a1 - a0) % 360.0) / 2.0)))
        d = (math.cos(mid), 0.0, math.sin(mid))
        tangent = (-d[2], 0.0, d[0])
        edge = (d[0] * EGG_R, 0.0, d[2] * EGG_R)
        xf = (B.translate(d[0] * out, 0.0, d[2] * out) @ B.about(edge, B.rot_axis(tangent, -tip))
              @ B.rot_y(spin) @ B.translate(0.0, EGG_BOTTOM, 0.0))
        if drop is None:
            # Rest the piece on the Ground plane as you see it: lower it until its lowest point on
            # screen is on the bottom row. A piece further back would otherwise look as if it floats.
            cam = B.camera(RENDER_ANCHOR)
            low = max(-cam.point(xf.point(q))[1] for q in _shard_points(a0, a1))
            drop = (CELL_H - 0.6 - low) / math.cos(math.radians(B.PITCH))
            drop = -drop
        xf = B.translate(0.0, -drop, 0.0) @ xf
        shell = B.Shell(EGG_R, EGG_TOP, EGG_BOTTOM, SHELL_T, crack, keep_top=False, angles=(a0, a1))
        part(f"shard_{n}", shell, xf, SHELL, f"shard{n}", "t", bands=(0.25, 0.70, 0.97))
    if p["shell_top"]:
        x, y, z, rot = p["shell_top"]
        shell = B.Shell(EGG_R, EGG_TOP, EGG_BOTTOM, SHELL_T, crack, keep_top=True)
        part("shell_top", shell, B.translate(x, y, z) @ B.rot_z(rot), SHELL, "shellt", "t", bands=(0.25, 0.70, 0.97))

    return parts, dict(root=root, torso=torso_xf, head=head_xf, tail=tail, tail_xf=tail_xf)


def _shard_points(a0, a1):
    """Points on the outside of a lower-shell piece, in egg space, to find where it touches the ground."""
    pts = []
    span = (a1 - a0) % 360.0
    for k in range(13):
        ang = a0 + span * k / 12.0
        a = math.radians(ang)
        top = crack(ang)
        for j in range(13):
            y = -EGG_BOTTOM + (top + EGG_BOTTOM) * j / 12.0
            h = EGG_TOP if y >= 0 else EGG_BOTTOM
            rr = EGG_R * math.sqrt(max(0.0, 1 - (y / h) ** 2))
            pts.append((rr * math.cos(a), y, rr * math.sin(a)))
    return pts


# ------------------------------------------------------------------ shading

SHELL_NAMES = ("helmet", "shell_top", "shell_bottom", "egg", "shard_0", "shard_1", "shard_2", "shard_3")


def shade(hit):
    """Palette key for one ray hit, and whether later passes must leave it alone."""
    part, t, n, face, local_p = hit[:5]
    shadow = len(hit) > 5
    if part.name in SHELL_NAMES:
        if face == "inner":
            return ("s" if B.dot(n, B.LIGHT) < 0.35 else "t"), False
        if face == "rim":
            return "v", False
        if face == "hole":
            return "s", False
        if any(B.dot(B.sub(local_p, sp), B.sub(local_p, sp)) < 0.55 for sp in SPECKLE_PTS):
            return "t", True
        return part.ramp[B.level(n, part.bands)], False
    k = B.level(n, part.bands)
    if part.name in ("torso", "belly"):
        r = part.shape.r
        nx = local_p[0] / r[0]
        nz = local_p[2] / r[2]
        # belly plates on the front of the chest and belly, striped every 3 pixels of torso height
        if nx > 0.42 and nz > -0.5:
            stripe = (local_p[1] + 30.0) % 3.0 < 0.9
            return ("b" if stripe and k < 2 else AMBER[min(k, 2)]), False
    if shadow:
        k = max(0, k - 2)
    return part.ramp[min(k, len(part.ramp) - 1)], False


# ------------------------------------------------------------------ lines and cleanup

def line_pass(hits, width, height):
    keys, groups, depth, keep = {}, {}, {}, set()
    for q, hit in hits.items():
        k, protected = shade(hit)
        keys[q], groups[q], depth[q] = k, hit[0].group, hit[1]
        if protected:
            keep.add(q)
    grid = [[px.TRANSPARENT] * width for _ in range(height)]
    for (x, y), k in keys.items():
        out = inner = False
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            q = (x + dx, y + dy)
            if q not in keys:
                out = True
            elif groups[q] != groups[(x, y)] and depth[q] > depth[(x, y)] + 1.5:
                inner = True
        if out:
            k = "o"
        elif inner:
            k = hits[(x, y)][0].line
        grid[y][x] = k
    _orphans(grid, groups, keep)
    close_gaps(grid)
    return grid


def _orphans(grid, groups, keep):
    """A shading pixel unlike all four neighbours, which all agree, takes their colour."""
    h, w = len(grid), len(grid[0])
    change = []
    for y in range(1, h - 1):
        for x in range(1, w - 1):
            k = grid[y][x]
            if k in (px.TRANSPARENT, "o") or (x, y) in keep:
                continue
            n = [grid[y][x + 1], grid[y][x - 1], grid[y + 1][x], grid[y - 1][x]]
            if len(set(n)) == 1 and n[0] != k and n[0] not in (px.TRANSPARENT, "o") \
                    and FAMILY_DARK.get(n[0]) == FAMILY_DARK.get(k):
                change.append((x, y, n[0]))
    for x, y, k in change:
        grid[y][x] = k


def close_gaps(grid, passes=2):
    """Fills 1-pixel transparent gaps between outlines, then turns any outline pixel that ended up
    inside the figure into the darkest colour of the part around it."""
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
                thin.append((x, y, max(sorted(set(fams)), key=fams.count) if fams else "1"))
        if not thin:
            break
        for x, y, k in thin:
            grid[y][x] = k


# ------------------------------------------------------------------ stamps on projected points

def _project(pt, anchor=RENDER_ANCHOR):
    c = B.camera(anchor).point(pt)
    return c[0], -c[1], 1000.0 - c[2]


def _visible(hits, x, y, t, groups, slack=2.0):
    hit = hits.get((int(math.floor(x)), int(math.floor(y))))
    return hit is not None and hit[0].group in groups and abs(hit[1] - t) <= slack


def _stamp(grid, rows, x, y):
    """Lays hand-drawn pixels on the grid. Returns how many fell outside the cell."""
    lost = 0
    for j, line in enumerate(rows):
        for i, ch in enumerate(line):
            gx, gy = x + i, y + j
            if ch == px.TRANSPARENT:
                continue
            if 0 <= gy < len(grid) and 0 <= gx < len(grid[0]):
                grid[gy][gx] = ch
            else:
                lost += 1
    return lost


def _line(x0, y0, x1, y1):
    """Pixels of a 1-pixel line, end points included."""
    pts = []
    dx, dy = abs(x1 - x0), -abs(y1 - y0)
    sx, sy = (1 if x0 < x1 else -1), (1 if y0 < y1 else -1)
    err = dx + dy
    while True:
        pts.append((x0, y0))
        if x0 == x1 and y0 == y1:
            return pts
        e2 = 2 * err
        if e2 >= dy:
            err += dy
            x0 += sx
        if e2 <= dx:
            err += dx
            y0 += sy


# The mouth runs along the lower snout, from its tip back towards the near cheek, as directions from
# the snout's centre. Each expression bends the back end (the corner) up or down.
MOUTH_LINE = [(0.97, -0.12, 0.10), (0.80, -0.16, 0.50), (0.50, -0.18, 0.82), (0.18, -0.15, 0.95)]
MOUTH_CORNER = {"smirk": 0.30, "flat": 0.0, "fang": 0.25, "grin": 0.45, "open": 0.0, "o": None, "none": None}


def _mouth(grid, hits, snout_xf, kind):
    corner = MOUTH_CORNER[kind]
    h, w = len(grid), len(grid[0])

    def at(direction):
        x, y, t = _project(snout_xf.point(B.surface_point(SNOUT_R, direction)))
        return x, y, t

    if kind == "o":
        x, y, t = at((0.85, -0.38, 0.45))
        if _visible(hits, x, y, t, ("head",), 3.0):
            _stamp(grid, [".o.", "oao", ".o."], int(x) - 1, int(y) - 1)
        return
    if corner is None:
        return
    dirs = [list(d) for d in MOUTH_LINE]
    dirs[-1][1] += corner          # raise the corner
    dirs[-2][1] += corner * 0.4
    pts = []
    for d in dirs:
        x, y, t = at(tuple(d))
        if _visible(hits, x, y, t, ("head",), 3.0):
            pts.append((int(x), int(y)))
    line = []
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        line += _line(x0, y0, x1, y1)
    for (x, y) in line:
        if 0 <= x < w and 0 <= y < h and grid[y][x] in "12345":
            grid[y][x] = "o"
    if not pts:
        return
    # details hang just under the line, a little back from the tip
    ref = line[len(line) // 3] if line else pts[0]
    def put(dx, dy, key, under=("12345",)):
        x, y = ref[0] + dx, ref[1] + dy
        if 0 <= x < w and 0 <= y < h and grid[y][x] in "12345":
            grid[y][x] = key
    if kind == "fang":
        put(0, 1, "w")
    elif kind == "grin":
        put(-1, 1, "w")
        put(1, 1, "w")
    elif kind == "open":
        for dx in (-1, 0, 1):
            put(dx, 1, "a")
        for dx in (-1, 0, 1):
            put(dx, 2, "o")


def stamps(grid, hits, p, xfs):
    head = xfs["head"]
    snout_xf = head @ B.translate(*SNOUT_AT)
    # eyes: the near one is a full stamp; the far one shows only when the head turns
    eye = EYES[p["eye"]]
    pt = head.point(B.surface_point(HEAD_R, (8.6, -0.6, 4.8)))
    x, y, t = _project(pt)
    if _visible(hits, x, y, t, ("head",), 2.5):
        _stamp(grid, eye, int(x) - len(eye[0]) // 2, int(y) - 1)
    far = head.point(B.surface_point(HEAD_R, (8.6, 0.8, -4.8)))
    x, y, t = _project(far)
    if _visible(hits, x, y, t, ("head",), 1.2) and p["eye"] not in ("closed", "happy"):
        _stamp(grid, [r[-2:] for r in eye[:-1]], int(x) - 1, int(y) - 1)
    _mouth(grid, hits, snout_xf, p["mouth"])
    # nostril
    pt = snout_xf.point(B.surface_point(SNOUT_R, (0.85, 0.35, 0.4)))
    x, y, t = _project(pt)
    if _visible(hits, x, y, t, ("head",), 2.0):
        _stamp(grid, ["o"], int(x), int(y))
    if p["blush"]:
        pt = head.point(B.surface_point(HEAD_R, (5.5, -4.0, 9.0)))
        x, y, t = _project(pt)
        if _visible(hits, x, y, t, ("head",), 2.5):
            _stamp(grid, ["pp"], int(x) - 1, int(y))
    # Where the overlay goes: the flame on the tail tip, the puff at the nose, and the effects.
    tip = xfs["tail_xf"].point(xfs["tail"][-1])
    x, y, _ = _project(tip)
    info = {"flame": (int(round(x)), int(round(y))), "effects": []}
    if p["puff"]:
        x, y, _ = _project(snout_xf.point((8.0, 5.0, 2.0)))
        info["effects"].append(("puff", int(x) + p["puff"][0], int(y) + p["puff"][1]))
    for name, pt in p["effects"]:
        x, y, _ = _project(xfs["root"].point(pt))
        info["effects"].append((name, int(round(x)), int(round(y))))
    for name, x, bottom in p["fx"]:
        h = len(EFFECTS[name])
        info["effects"].append((name, x, bottom - (h - 1) + h // 2))
    return info


def overlay(grid, info, flame=0, flame_kind="normal"):
    """Lays the overlay on a finished base Frame: effects, then the tail flame on top. Done after the
    paint-over, so a long hold can flicker its flame without a correction per flicker Frame."""
    rows = [list(r) for r in grid]
    lost = 0
    for name, x, y in info["effects"]:
        art = EFFECTS[name]
        lost += _stamp(rows, art, x - len(art[0]) // 2, y - len(art) // 2)
    if flame is not None and info.get("flame"):
        x, y = info["flame"]
        art = (FLAMES if flame_kind == "normal" else DIM_FLAMES)
        art = art[flame % len(art)]
        lost += _stamp(rows, art, x - len(art[0]) // 2, y - len(art) + 1)
    if lost:
        raise ValueError(f"the overlay lost {lost} pixel(s) off the cell: a flame or effect is cut off")
    _check_edges(rows)
    return ["".join(r) for r in rows]


# ------------------------------------------------------------------ drawing

def _check_edges(grid, name="frame"):
    h, w = len(grid), len(grid[0])
    bad = [(x, y) for y in range(h) for x in range(w)
           if grid[y][x] != px.TRANSPARENT and (x == 0 or x == w - 1 or y == 0)]
    if bad:
        raise ValueError(f"{name}: art reaches the cell edge at {bad[:4]}; the cell is {w}x{h}")


def draw_base(p, name="frame"):
    """The base layer of a hatchling Frame, without the overlay. Returns (grid, overlay info)."""
    if p["egg"] is not None:
        grid = draw_egg(cell=(CELL_W, CELL_H), anchor=ANCHOR, name=name, **p["egg"])
        return grid, {"flame": None, "effects": []}
    parts, xfs = model(p)
    hits = B.render(parts, CELL_W, CELL_H + BELOW, RENDER_ANCHOR, shadow_from=("helmet",),
                    shadow_on=("head", "snout"), shadow_reach=2.0, smooth=("body", "head"))
    sunk = sorted({h[0].name for (x, y), h in hits.items() if y >= CELL_H})
    if sunk:
        raise ValueError(f"{name}: below the Ground plane: {', '.join(sunk)}")
    grid = line_pass(hits, CELL_W, CELL_H)
    info = stamps(grid, hits, p, xfs)
    _check_edges(grid, name)
    return ["".join(r) for r in grid], info


def draw(p, name="frame"):
    """A finished hatchling Frame from the rig alone: base layer plus overlay, no paint-over."""
    grid, info = draw_base(p, name)
    return overlay(grid, info, p["flame"], p["flame_kind"])


def draw_egg(rock=0.0, lift=0.0, sx=1.0, sy=1.0, glow=0, name="egg", cell=EGG_CELL, anchor=EGG_ANCHOR):
    """The egg alone. rock: degrees about its ground contact. glow: 0 none, 1 faint, 2 bright."""
    root = (B.translate(0.0, lift, 0.0) @ B.about((0.0, 0.0, 0.0), B.rot_z(-rock)) @ B.scale(sx, sy, sx)
            @ B.translate(0.0, EGG_BOTTOM, 0.0))
    shell = B.Shell(EGG_R, EGG_TOP, EGG_BOTTOM, SHELL_T, crack, keep_top=None)
    part = B.Part("egg", shell, root, SHELL, "egg", "t", bands=list(SHELL_BANDS))
    hits = B.render([part], cell[0], cell[1], anchor)
    grid = [[px.TRANSPARENT] * cell[0] for _ in range(cell[1])]
    seam_key = {0: "t", 1: "b", 2: "y"}[glow]
    for (x, y), hit in hits.items():
        local = hit[4]
        k, _ = shade(hit)
        cy = shell.crack_y(local)
        if abs(local[1] - cy) < 0.75:
            k = seam_key
        elif glow == 2 and 0 <= cy - local[1] < 1.6:
            k = "c"
        grid[y][x] = k
    for (x, y) in list(hits):
        if any((x + dx, y + dy) not in hits for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))):
            grid[y][x] = "o"
    out = ["".join(r) for r in grid]
    _check_edges([list(r) for r in out], name)
    return out
