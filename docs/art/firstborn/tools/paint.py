"""Hand-correct one drawing of the Pack.

    py docs/art/firstborn/tools/paint.py <form>/<animation> <drawing> x,y=key [x,y=key ...]

<drawing> counts from 1, as in 03-timing-sheets.md. A key is a palette key, '_' to erase the base
pixel, or '.' to drop an earlier correction. Coordinates are cell pixels from the top-left.

The correction goes in paintover/shared.txt when the drawing's pose is drawn more than once (REST,
SIT, SLEEP, the still egg, or any other repeat), so it shows the same everywhere; otherwise in
paintover/<form>/<animation>.txt. The Frame records the fingerprint of the base it was painted on.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, ROOT)

import export as X  # noqa: E402
import paintover as po  # noqa: E402
import pixkit as px  # noqa: E402


def target(form, anim_name, number):
    """(paint-over file, Frame name, base grid) for drawing `number` of an Animation."""
    anim = next(a for a in X.FORMS[form]["anims"] if a.name == anim_name)
    base_form, pose = X.drawing_pose(form, anim.frames[number - 1].pose)
    key = X.T.pose_key({"form": base_form, "pose": pose})
    grid, _ = X.T.base(pose, f"{form}/{anim_name}", base_form)
    if key in X.SHARED:
        return os.path.join(X.PAINT, "shared.txt"), X.SHARED[key][0], grid
    return os.path.join(X.PAINT, form, f"{anim_name}.txt"), f"d{number - 1:02d}", grid


def main(argv):
    form, anim_name = argv[0].split("/")
    path, name, base = target(form, anim_name, int(argv[1]))
    found = po.read(path)
    w, h = px.grid_size(base)
    digest = po.base_hash(base)
    if name in found and not po.is_empty(found[name][1]) and found[name][0] != digest:
        raise SystemExit(f"{path}: {name} was painted on an older base; redo it, starting from '.'s")
    rows = [list(r) for r in (found[name][1] if name in found else [px.TRANSPARENT * w] * h)]
    for e in argv[2:]:
        xy, k = e.split("=")
        x, y = (int(v) for v in xy.split(","))
        if not (0 <= x < w and 0 <= y < h) or len(k) != 1:
            raise SystemExit(f"bad edit {e}")
        rows[y][x] = k
    found[name] = (digest, ["".join(r) for r in rows])
    po.write_raw(path, [(n, len(g[0]), len(g), d, g) for n, (d, g) in found.items()])
    print(f"{path}: {name}: {len(argv) - 2} pixel(s) set")


if __name__ == "__main__":
    main(sys.argv[1:])
