"""Builds the firstborn Pack: strips, manifest and checks.

    py docs/art/firstborn/export.py              build into .build/firstborn and check it
    py docs/art/firstborn/export.py --publish    ...then copy it to content/firstborn if every check passes

Hand corrections are made with tools/paint.py (see its docstring) and live in paintover/.

The steps, all in this script so they're the same every time:
1. Every drawing of every timing sheet (anim/egg.py, anim/hatchling.py) gets its base layer from the
   rig, cached by pose.
2. The paint-over is laid on: paintover/shared.txt for the poses several Animations share (REST, SIT,
   SLEEP, the still egg), else paintover/<form>/<animation>.txt. A paint-over Frame whose base has
   changed stops the export.
3. The hatch's egg drawings are the egg Form's own finished Frames, moved into the hatchling cell, so
   switching Form can't show.
4. The flame and effects go on top, and long drawings are split so the flame keeps flickering.
5. Checks: stray pixels, gaps in the outline, art on the cell edge, grounded poses that don't touch
   the ground, the hatch matching the egg, and the same pose looking the same everywhere.
6. Bodies are measured from the art: the top of each is the top of the helmet dome.
7. The strips, Emotes and manifest.toml are written, and the Pack is checked against the 11
   validator rules from docs/proposals/item-3-architecture.md.
"""

import os
import shutil
import sys
import tomllib

ROOT = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(ROOT, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools"))
sys.path.insert(0, os.path.join(ROOT, "anim"))

import brim  # noqa: E402
import egg  # noqa: E402
import emotes  # noqa: E402
import hatchling  # noqa: E402
import paintover as po  # noqa: E402
import pixkit as px  # noqa: E402
import timing as T  # noqa: E402

STAGE = os.path.join(ROOT, ".build", "firstborn")
CONTENT = os.path.join(REPO, "content", "firstborn")
PAINT = os.path.join(ROOT, "paintover")

PACK_NAME = "Brim"
SHADOW = {"width": 30, "colour": "#1a132660"}   # the same on both Forms, so the hatch can't show a jump
SHELL_KEYS = set("stuvw")

FORMS = {
    "egg": dict(anims=egg.ANIMATIONS, cell=brim.EGG_CELL, anchor=brim.EGG_ANCHOR, default="idle",
                actions={"idle": ("idle", None), "petted": ("wobble", None)},
                situations={"held": "held", "falling": "falling", "landing": "landing"}),
    "hatchling": dict(anims=hatchling.ANIMATIONS, cell=(brim.CELL_W, brim.CELL_H), anchor=brim.ANCHOR,
                      default="idle",
                      actions={"idle": ("idle", None), "idle_helmet": ("idle_helmet", None),
                               "idle_puff": ("idle_puff", None), "walk": ("walk", None), "run": ("run", None),
                               "follow": ("run", None), "flee": ("run", None), "sit": ("sit", "sit"),
                               "sleep": ("sleep", "sleep"), "look": ("look", None), "pounce": ("pounce", None),
                               "poke": ("poke", None), "petted": ("petted", None), "hatch": ("hatch", None)},
                      situations={"held": "held", "falling": "falling", "landing": "landing"}),
}

# Event markers, on the first exported Frame of the named drawing.
MARKERS = {
    ("hatchling", "walk"): [(0, "footstep"), (4, "footstep")],
    ("hatchling", "run"): [(0, "footstep"), (3, "footstep")],
    ("hatchling", "pounce"): [(8, "impact")],
    ("hatchling", "landing"): [(0, "impact")],
    ("hatchling", "hatch"): [(7, "hatch")],
    ("egg", "landing"): [(0, "impact")],
}

# Body widths: the Form's own, and the per-Action ones for the lower poses. The top is measured.
BODY_X = {("egg", None): (-14, 28), ("hatchling", None): (-14, 28), ("hatchling", "sit"): (-15, 30),
          ("hatchling", "sleep"): (-16, 32)}


def drawing_pose(form, pose):
    """The Form and pose a drawing's base is rendered from: the hatch's egg drawings are the egg's."""
    if form == "hatchling" and pose.get("egg") is not None:
        return "egg", {"egg": pose["egg"]}
    return form, pose


def shared_poses():
    """Every pose drawn more than once gets one correction slot in paintover/shared.txt, so a hand
    correction shows the same wherever the pose appears. The common ones have names; any other repeat
    is named after its first drawing, such as look_01."""
    named = {"REST": ("hatchling", hatchling.REST), "SIT": ("hatchling", brim.pose(**hatchling.SIT)),
             "SLEEP": ("hatchling", brim.pose(**hatchling.SLEEP)), "EGG_STILL": ("egg", {"egg": {}})}
    shared = {T.pose_key({"form": f, "pose": p}): (n, f, p) for n, (f, p) in named.items()}
    seen = {}
    for form, spec in FORMS.items():
        for a in spec["anims"]:
            for i, fr in enumerate(a.frames):
                f, p = drawing_pose(form, fr.pose)
                key = T.pose_key({"form": f, "pose": p})
                if key in seen and key not in shared:
                    shared[key] = (seen[key], f, p)
                seen.setdefault(key, f"{a.name}_{i + 1:02d}" if form == "hatchling" else f"egg_{a.name}_{i + 1:02d}")
    return shared


class Report:
    def __init__(self):
        self.errors, self.warnings, self.notes = [], [], []

    def error(self, msg):
        self.errors.append(msg)

    def warn(self, msg):
        self.warnings.append(msg)


# ------------------------------------------------------------------ drawings

def corrected(form, pose, report, where, anim_file=None, frame_name=None):
    grid, info = T.base(pose, where, form)
    key = T.pose_key({"form": form, "pose": pose})
    shared = SHARED.get(key)
    if shared:
        path, name = os.path.join(PAINT, "shared.txt"), shared[0]
    elif anim_file:
        path, name = anim_file, frame_name
    else:
        return grid, info
    try:
        grid = dict(po.apply(path, [(name, grid)]))[name]
    except ValueError as err:
        report.error(f"{where}: {err}")
    return grid, info


def place_in(grid, cell, anchor_from, anchor_to):
    """Moves a grid into a bigger cell so the Anchors line up."""
    dx, dy = anchor_to[0] - anchor_from[0], anchor_to[1] - anchor_from[1]
    return px.paste(px.blank(*cell), grid, dx, dy)


def build_animation(form, anim, report):
    """Returns (exported Frames as grids, ms per Frame, drawing grids, first exported Frame per drawing)."""
    spec = FORMS[form]
    anim_file = os.path.join(PAINT, form, f"{anim.name}.txt")
    drawings = []
    for i, f in enumerate(anim.frames):
        where = f"{form}/{anim.name} drawing {i + 1}"
        if form == "egg":
            grid, info = corrected("egg", f.pose, report, where, anim_file, f"d{i:02d}")
        elif f.pose.get("egg") is not None:
            # the hatch's egg: the egg Form's own finished Frame, moved into this cell
            egg_pose = {"egg": f.pose["egg"]}
            egg_grid, _ = corrected("egg", egg_pose, report, where, anim_file, f"d{i:02d}")
            grid, info = place_in(egg_grid, spec["cell"], brim.EGG_ANCHOR, brim.ANCHOR), {"flame": None, "effects": []}
        else:
            grid, info = corrected(form, f.pose, report, where, anim_file, f"d{i:02d}")
        drawings.append((grid, info))
    frames, first = [], {}
    sched = T.schedule(anim)
    for n, (i, flame, ticks) in enumerate(sched):
        first.setdefault(i, n)
        grid, info = drawings[i]
        if form == "hatchling":
            kind = anim.frames[i].flame_kind or anim.flame_kind
            try:
                grid = brim.overlay(grid, info, flame, kind)
            except ValueError as err:
                report.error(f"{form}/{anim.name} drawing {i + 1}: {err}")
        frames.append(grid)
    ms = T.ms_list([t for _, _, t in sched])
    return frames, ms, [g for g, _ in drawings], first


# ------------------------------------------------------------------ checks

def check_drawing(form, anim, i, grid, report):
    where = f"{form}/{anim.name} drawing {i + 1} ({anim.frames[i].label})"
    h, w = len(grid), len(grid[0])
    for y in range(h):
        for x in range(w):
            k = grid[y][x]
            if k == px.TRANSPARENT:
                continue
            nb = [grid[y + dy][x + dx] if 0 <= x + dx < w and 0 <= y + dy < h else px.TRANSPARENT
                  for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))]
            if all(n == px.TRANSPARENT for n in nb):
                report.warn(f"{where}: a stray pixel at ({x}, {y})")
            elif k != "o" and px.TRANSPARENT in nb:
                report.warn(f"{where}: a gap in the outline at ({x}, {y})")
            if x in (0, w - 1) or y == 0:
                report.error(f"{where}: art on the cell edge at ({x}, {y})")
    pose = anim.frames[i].pose
    airborne = (pose.get("lift", 0) or 0) > 0.5 or anim.name in ("held", "falling")
    if not airborne and not any(c != px.TRANSPARENT for c in grid[h - 1]):
        report.error(f"{where}: a grounded pose with nothing on the bottom row: it would float")


def dome_top(grids):
    """The highest row of helmet or egg shell in these Frames."""
    rows = [y for g in grids for y, r in enumerate(g) if any(c in SHELL_KEYS for c in r)]
    return min(rows) if rows else 0


# ------------------------------------------------------------------ writing

def write_strip(path, frames, palette):
    w, h = px.grid_size(frames[0])
    rows = [b"".join(r) for r in zip(*[px.grid_to_rgba(g, palette) for g in frames])]
    os.makedirs(os.path.dirname(path), exist_ok=True)
    px.write_png(path, w * len(frames), h, rows)


def toml_body(b):
    return f'{{ x = {b[0]}, y = {b[1]}, width = {b[2]}, height = {b[3]} }}'


def build(report):
    if os.path.exists(STAGE):
        shutil.rmtree(STAGE)
    os.makedirs(STAGE)
    manifest = ['# Brim, the firstborn character: a dragon who wears the top of its eggshell as a helmet.',
                '# Generated by docs/art/firstborn/export.py. Edit the sources there, not this file.',
                'format_version = 1', f'name = "{PACK_NAME}"', 'starting_form = "egg"', '']
    built = {}
    for form, spec in FORMS.items():
        anims = {a.name: build_animation(form, a, report) for a in spec["anims"]}
        built[form] = anims
        for a in spec["anims"]:
            frames, ms, drawings, first = anims[a.name]
            for i, g in enumerate(drawings):
                check_drawing(form, a, i, g, report)
        ax, ay = spec["anchor"]
        default_frames = [g for g in anims[spec["default"]][2]]
        top = dome_top(default_frames)
        bx, bw = BODY_X[(form, None)]
        body = (bx, top - ay, bw, ay - top)
        manifest += [f'[forms.{form}]', f'cell = {{ width = {spec["cell"][0]}, height = {spec["cell"][1]} }}',
                     f'anchor = {{ x = {ax}, y = {ay} }}', f'body = {toml_body(body)}',
                     f'default_action = "{spec["default"]}"',
                     f'shadow = {{ width = {SHADOW["width"]}, colour = "{SHADOW["colour"]}" }}', '']
        for action, (anim_name, body_for) in spec["actions"].items():
            manifest.append(f'[forms.{form}.actions.{action}]')
            manifest.append(f'animation = "{anim_name}"')
            if body_for:
                t = dome_top(anims[body_for][2])
                bx, bw = BODY_X[(form, body_for)]
                manifest.append(f'body = {toml_body((bx, t - ay, bw, ay - t))}')
            manifest.append('')
        manifest.append(f'[forms.{form}.situations]')
        for sit, anim_name in spec["situations"].items():
            manifest.append(f'{sit} = "{anim_name}"')
        manifest.append('')
        for a in spec["anims"]:
            frames, ms, drawings, first = anims[a.name]
            sheet = f"{form}/{a.name}.png"
            write_strip(os.path.join(STAGE, sheet), frames, brim.PALETTE)
            manifest.append(f'[forms.{form}.animations.{a.name}]')
            manifest.append(f'sheet = "{sheet}"')
            manifest.append(f'durations_ms = [{", ".join(str(m) for m in ms)}]')
            marks = MARKERS.get((form, a.name))
            if marks:
                manifest.append('markers = [' + ", ".join(f'{{ frame = {first[i]}, name = "{n}" }}' for i, n in marks)
                                + ']')
            manifest.append('')
    # the same pose must give the same final pixels everywhere it's drawn
    by_pose = {}
    for form, spec in FORMS.items():
        for a in spec["anims"]:
            for i, g in enumerate(built[form][a.name][2]):
                f, p = drawing_pose(form, a.frames[i].pose)
                g = g if form == f else [r[12:52] for r in g[32:80]]   # the hatch's egg, back in the egg's cell
                key = T.pose_key({"form": f, "pose": p})
                if key in by_pose and by_pose[key][1] != g:
                    report.error(f"{form}/{a.name} drawing {i + 1}: the same pose as {by_pose[key][0]}, "
                                 "but different pixels")
                by_pose.setdefault(key, (f"{form}/{a.name} drawing {i + 1}", g))
    report.notes.append(f"{len(by_pose)} different poses; every repeated one looks the same wherever it's drawn")
    # the hatch must start exactly as the egg does
    egg_first = built["egg"]["idle"][0][0]
    hatch_first = built["hatchling"]["hatch"][0][0]
    if place_in(egg_first, FORMS["hatchling"]["cell"], brim.EGG_ANCHOR, brim.ANCHOR) != hatch_first:
        report.error("hatchling/hatch: its first Frame doesn't match the egg Form's first Frame")
    else:
        report.notes.append("hatch's first Frame matches the egg Form's idle first Frame, pixel for pixel")
    # Emotes
    for e in emotes.EMOTES:
        name, mode, fr = e
        grids = emotes.frames(e)
        sheet = f"emotes/{name}.png"
        write_strip(os.path.join(STAGE, sheet), grids, brim.PALETTE)
        ms = T.ms_list([t for _, t, _ in fr])
        manifest += [f'[emotes.{name}]', f'sheet = "{sheet}"', 'cell = { width = 16, height = 16 }',
                     f'durations_ms = [{", ".join(str(m) for m in ms)}]', f'mode = "{mode}"', '']
    with open(os.path.join(STAGE, "manifest.toml"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(manifest))


# ------------------------------------------------------------------ the validator rules, applied to the files

def validate(folder):
    errors = []
    try:
        with open(os.path.join(folder, "manifest.toml"), "rb") as f:
            m = tomllib.load(f)
    except (OSError, tomllib.TOMLDecodeError) as err:
        return [f"manifest.toml: {err}"]
    top_fields = {"format_version", "name", "starting_form", "hit_mask_threshold", "forms", "emotes"}
    for k in set(m) - top_fields:
        errors.append(f"{k}: isn't a field")
    if m.get("format_version") != 1:
        errors.append("format_version: must be 1")
    threshold = m.get("hit_mask_threshold", 128)
    if not 1 <= threshold <= 255:
        errors.append("hit_mask_threshold: must be 1-255")
    forms = m.get("forms", {})
    if not forms or m.get("starting_form") not in forms:
        errors.append("starting_form: must name a Form")
    form_fields = {"cell", "anchor", "body", "default_action", "shadow", "actions", "situations", "animations"}
    for fname, form in forms.items():
        path = f"forms.{fname}"
        for k in set(form) - form_fields:
            errors.append(f"{path}.{k}: isn't a field")
        cell, anchor = form["cell"], form["anchor"]
        if not (0 <= anchor["x"] <= cell["width"] and 0 <= anchor["y"] <= cell["height"]):
            errors.append(f"{path}.anchor: outside the cell")
        actions, anims, sits = form.get("actions", {}), form.get("animations", {}), form.get("situations", {})
        if form["default_action"] not in actions:
            errors.append(f"{path}.default_action: not one of its Actions")
        for b in [form["body"]] + [a["body"] for a in actions.values() if "body" in a]:
            if b["width"] <= 0 or b["height"] <= 0:
                errors.append(f"{path}: a Body without a positive size")
        used = set()
        for aname, a in actions.items():
            for k in set(a) - {"animation", "body"}:
                errors.append(f"{path}.actions.{aname}.{k}: isn't a field")
            if a["animation"] not in anims:
                errors.append(f"{path}.actions.{aname}: names no Animation of this Form")
            used.add(a["animation"])
        for sname, aname in sits.items():
            if sname not in ("held", "falling", "landing"):
                errors.append(f"{path}.situations.{sname}: isn't a Situation")
            if aname not in anims:
                errors.append(f"{path}.situations.{sname}: names no Animation of this Form")
            used.add(aname)
        for aname, a in anims.items():
            apath = f"{path}.animations.{aname}"
            for k in set(a) - {"sheet", "durations_ms", "markers"}:
                errors.append(f"{apath}.{k}: isn't a field")
            if aname not in used:
                errors.append(f"{apath}: isn't played by any Action or Situation")
            errors += check_sheet(folder, a["sheet"], cell, a["durations_ms"], threshold, apath)
            for mk in a.get("markers", []):
                if not 0 <= mk["frame"] < len(a["durations_ms"]):
                    errors.append(f"{apath}.markers: Frame {mk['frame']} doesn't exist")
    for ename, e in m.get("emotes", {}).items():
        epath = f"emotes.{ename}"
        for k in set(e) - {"sheet", "cell", "durations_ms", "mode"}:
            errors.append(f"{epath}.{k}: isn't a field")
        if e["mode"] not in ("one-shot", "looping"):
            errors.append(f"{epath}.mode: must be one-shot or looping")
        errors += check_sheet(folder, e["sheet"], e["cell"], e["durations_ms"], None, epath)
    return errors


def check_sheet(folder, sheet, cell, durations, threshold, path):
    errors = []
    full = os.path.join(folder, sheet)
    if not os.path.exists(full):
        return [f"{path}.sheet: {sheet} doesn't exist"]
    w, h, ctype, rows = px.read_png(full)
    if ctype != 6:
        errors.append(f"{path}.sheet: not a PNG with transparency")
    if h != cell["height"] or w != cell["width"] * len(durations):
        errors.append(f"{path}.sheet: {w}x{h}, but should be {cell['width'] * len(durations)}x{cell['height']}")
    if any(d <= 0 for d in durations) or not durations:
        errors.append(f"{path}.durations_ms: every duration must be above 0")
    if threshold is not None and not errors:
        cw = cell["width"]
        for n in range(len(durations)):
            if not any(r[(n * cw + x) * 4 + 3] >= threshold for r in rows for x in range(cw)):
                errors.append(f"{path}: Frame {n} has no pixel the Pet could be clicked on")
    for r in rows:
        for x in range(w):
            a = r[x * 4 + 3]
            if a not in (0, 255):
                errors.append(f"{path}.sheet: a half-transparent pixel; the art must be opaque or clear")
                return errors
    return errors


SHARED = shared_poses()


def main():
    report = Report()
    build(report)
    report.errors += validate(STAGE)
    with open(os.path.join(ROOT, ".build", "report.txt"), "w", encoding="utf-8") as f:
        f.write("\n".join(["errors:"] + report.errors + ["", "warnings:"] + report.warnings + ["", "notes:"]
                          + report.notes) + "\n")
    print(f"{len(report.errors)} errors, {len(report.warnings)} warnings. Report: .build/report.txt")
    for e in report.errors[:20]:
        print("  error:", e)
    for n in report.notes:
        print("  ", n)
    if report.errors:
        sys.exit(1)
    if "--publish" in sys.argv:
        if os.path.exists(CONTENT):
            shutil.rmtree(CONTENT)
        shutil.copytree(STAGE, CONTENT)
        print("published to", CONTENT)


if __name__ == "__main__":
    main()
