"""pixkit: a small pixel-art toolkit for the firstborn Pack, standard-library Python only.

Run with `py` (Python 3.10 or later). Nothing to install.

The pieces:
- Grids: a Frame is a list of strings, one character per pixel. '.' is transparent; every other
  character is a key in a palette. Grids are the editable source of truth.
- Figures: a pose is built from Parts (ellipses, polygons, strokes) drawn back to front. The
  Figure shades each Part, draws its outline, and returns a Grid. Hand-drawn stamps go on top.
- PNG: write_png and read_png handle 8-bit RGBA, enough for strips, previews and validation.
"""

import math
import struct
import zlib

TRANSPARENT = "."


# ---------------------------------------------------------------- palette and grids

def hex_rgba(value):
    value = value.lstrip("#")
    r, g, b = int(value[0:2], 16), int(value[2:4], 16), int(value[4:6], 16)
    a = int(value[6:8], 16) if len(value) == 8 else 255
    return (r, g, b, a)


def blank(width, height):
    return [TRANSPARENT * width for _ in range(height)]


def grid_size(grid):
    return (len(grid[0]) if grid else 0, len(grid))


def flip_x(grid):
    return [row[::-1] for row in grid]


def paste(base, top, x, y):
    """Copy every non-transparent pixel of `top` onto `base` at (x, y). Returns a new grid."""
    rows = [list(r) for r in base]
    h, w = len(rows), len(rows[0])
    for j, line in enumerate(top):
        for i, ch in enumerate(line):
            if ch != TRANSPARENT and 0 <= x + i < w and 0 <= y + j < h:
                rows[y + j][x + i] = ch
    return ["".join(r) for r in rows]


def silhouette(grid, key="#"):
    return ["".join(key if ch != TRANSPARENT else TRANSPARENT for ch in row) for row in grid]


def grid_to_rgba(grid, palette):
    """Palette maps key -> '#rrggbb'. Returns rows of bytes, 4 bytes per pixel."""
    lookup = {k: bytes(hex_rgba(v)) for k, v in palette.items()}
    lookup[TRANSPARENT] = bytes((0, 0, 0, 0))
    out = []
    for row in grid:
        try:
            out.append(b"".join(lookup[ch] for ch in row))
        except KeyError as err:
            raise KeyError(f"pixel key {err} is not in the palette") from None
    return out


# ---------------------------------------------------------------- grid files

def write_grid_file(path, frames, palette, note=""):
    """frames: list of (name, grid). A text file a person can edit by hand."""
    lines = ["# firstborn pixel grid, format 1"]
    if note:
        lines += [f"# {n}" for n in note.splitlines()]
    lines.append("palette")
    for k, v in palette.items():
        lines.append(f"  {k} {v}")
    for name, grid in frames:
        w, h = grid_size(grid)
        lines.append(f"frame {name} {w}x{h}")
        lines += grid
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines) + "\n")


def read_grid_file(path):
    """Returns (palette, [(name, grid)]). Lines starting with '#' are comments."""
    palette, frames = {}, []
    mode, current, want = None, None, (0, 0)
    with open(path, encoding="utf-8") as f:
        for n, raw in enumerate(f, 1):
            line = raw.rstrip("\n")
            if line.startswith("#") or (not line.strip() and mode != "frame"):
                continue
            if line == "palette":
                mode = "palette"
                continue
            if line.startswith("frame "):
                _, name, size = line.split()
                w, h = (int(v) for v in size.split("x"))
                current, want, mode = (name, []), (w, h), "frame"
                frames.append(current)
                continue
            if mode == "palette":
                k, v = line.split()
                palette[k] = v
            elif mode == "frame":
                if len(line) != want[0]:
                    raise ValueError(f"{path}:{n}: row is {len(line)} wide, frame {current[0]} is {want[0]}")
                current[1].append(line)
    for name, grid in frames:
        if len(grid) == 0:
            raise ValueError(f"{path}: frame {name} has no rows")
    return palette, frames


# ---------------------------------------------------------------- PNG

def _chunk(tag, data):
    body = tag + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def write_png(path, width, height, rows):
    raw = b"".join(b"\x00" + r for r in rows)
    data = (b"\x89PNG\r\n\x1a\n"
            + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + _chunk(b"IDAT", zlib.compress(raw, 9))
            + _chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(data)


def png_bytes(width, height, rows):
    raw = b"".join(b"\x00" + r for r in rows)
    return (b"\x89PNG\r\n\x1a\n"
            + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
            + _chunk(b"IDAT", zlib.compress(raw, 9))
            + _chunk(b"IEND", b""))


def read_png(path):
    """Returns (width, height, colour_type, rows of RGBA bytes). 8-bit, non-interlaced only."""
    with open(path, "rb") as f:
        data = f.read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG")
    pos, idat, ihdr = 8, b"", None
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos:pos + 4])
        tag = data[pos + 4:pos + 8]
        body = data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if tag == b"IHDR":
            ihdr = struct.unpack(">IIBBBBB", body)
        elif tag == b"IDAT":
            idat += body
        elif tag == b"IEND":
            break
    width, height, depth, ctype, _, _, interlace = ihdr
    if depth != 8 or interlace != 0 or ctype not in (2, 6):
        raise ValueError(f"unsupported PNG: depth {depth}, colour type {ctype}, interlace {interlace}")
    bpp = 4 if ctype == 6 else 3
    raw = zlib.decompress(idat)
    stride = width * bpp
    rows, prev = [], bytearray(stride)
    for y in range(height):
        ftype = raw[y * (stride + 1)]
        line = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if ftype == 1:
                line[i] = (line[i] + a) & 255
            elif ftype == 2:
                line[i] = (line[i] + b) & 255
            elif ftype == 3:
                line[i] = (line[i] + (a + b) // 2) & 255
            elif ftype == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        prev = line
        if bpp == 3:
            line = bytearray(b"".join(bytes(line[i:i + 3]) + b"\xff" for i in range(0, stride, 3)))
        rows.append(bytes(line))
    return width, height, ctype, rows


# ---------------------------------------------------------------- images (RGBA rows)

def rgba_scale(rows, factor):
    out = []
    for r in rows:
        line = b"".join(r[i:i + 4] * factor for i in range(0, len(r), 4))
        out += [line] * factor
    return out


def rgba_canvas(width, height, colour=(0, 0, 0, 0)):
    return [bytearray(bytes(colour) * width) for _ in range(height)]


def rgba_blit(canvas, rows, x, y):
    """Alpha-over blit (binary alpha is all the art uses, so a simple over is enough)."""
    for j, r in enumerate(rows):
        if not 0 <= y + j < len(canvas):
            continue
        line = canvas[y + j]
        for i in range(0, len(r), 4):
            px = x + i // 4
            if 0 <= px < len(line) // 4 and r[i + 3] >= 128:
                line[px * 4:px * 4 + 4] = r[i:i + 4]


def rgba_fill(canvas, x, y, w, h, colour):
    for j in range(max(0, y), min(len(canvas), y + h)):
        line = canvas[j]
        for i in range(max(0, x), min(len(line) // 4, x + w)):
            line[i * 4:i * 4 + 4] = bytes(colour)


# ---------------------------------------------------------------- shapes (sets of pixels)

def ellipse(cx, cy, rx, ry):
    pts = set()
    for y in range(math.floor(cy - ry) - 1, math.ceil(cy + ry) + 2):
        for x in range(math.floor(cx - rx) - 1, math.ceil(cx + rx) + 2):
            dx, dy = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
            if dx * dx + dy * dy <= 1.0:
                pts.add((x, y))
    return pts


def egg(cx, cy, rx, ry, taper=0.18):
    """An egg: narrower at the top. `taper` is how much narrower."""
    pts = set()
    for y in range(math.floor(cy - ry) - 1, math.ceil(cy + ry) + 2):
        v = (y + 0.5 - cy) / ry
        if abs(v) > 1:
            continue
        local_rx = rx * (1 - taper * (-v)) if v < 0 else rx * (1 - taper * 0.15 * v)
        span = local_rx * math.sqrt(max(0.0, 1 - v * v))
        for x in range(math.floor(cx - rx) - 1, math.ceil(cx + rx) + 2):
            if abs(x + 0.5 - cx) <= span:
                pts.add((x, y))
    return pts


def polygon(points):
    pts = set()
    xs, ys = [p[0] for p in points], [p[1] for p in points]
    for y in range(math.floor(min(ys)), math.ceil(max(ys)) + 1):
        for x in range(math.floor(min(xs)), math.ceil(max(xs)) + 1):
            if _inside(points, x + 0.5, y + 0.5):
                pts.add((x, y))
    return pts


def _inside(poly, x, y):
    inside, n = False, len(poly)
    for i in range(n):
        x1, y1 = poly[i]
        x2, y2 = poly[(i + 1) % n]
        if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1) + x1:
            inside = not inside
    return inside


def stroke(path, radius):
    """A thick line through the points of `path`, `radius` may be a number or a list per point."""
    pts = set()
    radii = radius if isinstance(radius, (list, tuple)) else [radius] * len(path)
    for (x1, y1), (x2, y2), r1, r2 in zip(path, path[1:], radii, radii[1:]):
        steps = max(1, int(math.hypot(x2 - x1, y2 - y1) * 3))
        for s in range(steps + 1):
            t = s / steps
            pts |= ellipse(x1 + (x2 - x1) * t, y1 + (y2 - y1) * t, r1 + (r2 - r1) * t, r1 + (r2 - r1) * t)
    return pts


def rect(x, y, w, h):
    return {(i, j) for j in range(y, y + h) for i in range(x, x + w)}


def from_rows(rows, x, y, key=None):
    """Pixels of a small hand-drawn mask: any non-'.' character counts."""
    return {(x + i, y + j) for j, r in enumerate(rows) for i, ch in enumerate(r) if ch != TRANSPARENT
            and (key is None or ch == key)}


# ---------------------------------------------------------------- figures

class Part:
    """One piece of a figure, shaded with a ramp of palette keys from dark to light.

    shade: 'sphere' shades like a ball lit from above; 'rim' lights the top edge and darkens the
    bottom edge; 'flat' uses ramp[1] (or ramp[0] if there is one key).
    line:  the key used where this part's edge lies over a part behind it. None draws no line.
    group: parts in the same group draw no line between each other.
    """

    def __init__(self, pixels, ramp, shade="sphere", line=None, group=None, centre=None, radii=None,
                 outline=None, light_bias=0.0):
        self.pixels = set(pixels)
        self.ramp = ramp
        self.shade = shade
        self.line = line
        self.group = group
        self.centre = centre
        self.radii = radii
        self.outline = outline
        self.light_bias = light_bias


LIGHT = (0.0, -0.78, 0.62)   # from above and a little in front: reads the same when mirrored


def _sphere_level(part, x, y, levels):
    cx, cy = part.centre
    rx, ry = part.radii
    u, v = (x + 0.5 - cx) / rx, (y + 0.5 - cy) / ry
    nz = math.sqrt(max(0.0, 1 - u * u - v * v))
    lx, ly, lz = LIGHT
    i = u * lx + v * ly + nz * lz + part.light_bias
    # bands: tuned so most of a ball is the base colour, a cap is light, an underside is dark
    if levels == 2:
        return 0 if i < 0.30 else 1
    if levels == 3:
        return 0 if i < 0.28 else 1 if i < 0.82 else 2
    return 0 if i < 0.22 else 1 if i < 0.66 else 2 if i < 0.93 else 3


class Figure:
    def __init__(self, width, height, outline_key):
        self.width, self.height = width, height
        self.outline_key = outline_key
        self.parts = []
        self.stamps = []

    def add(self, part):
        self.parts.append(part)
        return part

    def stamp(self, rows, x, y):
        """Hand-drawn pixels laid over the finished figure; '.' leaves a pixel alone."""
        self.stamps.append((rows, x, y))

    def render(self):
        w, h = self.width, self.height
        owner = [[-1] * w for _ in range(h)]
        key = [[TRANSPARENT] * w for _ in range(h)]
        for idx, part in enumerate(self.parts):
            if part.shade == "sphere" and part.centre is None:
                xs = [p[0] for p in part.pixels]
                ys = [p[1] for p in part.pixels]
                part.centre = ((min(xs) + max(xs) + 1) / 2, (min(ys) + max(ys) + 1) / 2)
                part.radii = (max(1.0, (max(xs) - min(xs) + 1) / 2), max(1.0, (max(ys) - min(ys) + 1) / 2))
            for (x, y) in part.pixels:
                if not (0 <= x < w and 0 <= y < h):
                    continue
                owner[y][x] = idx
                key[y][x] = self._shade(part, x, y)
        result = [row[:] for row in key]
        for y in range(h):
            for x in range(w):
                idx = owner[y][x]
                if idx < 0:
                    continue
                part = self.parts[idx]
                edge_out, edge_in = False, False
                for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    nx, ny = x + dx, y + dy
                    other = owner[ny][nx] if 0 <= nx < w and 0 <= ny < h else -1
                    if other < 0:
                        edge_out = True
                    elif other < idx and part.line is not None:
                        o = self.parts[other]
                        if part.group is None or o.group != part.group:
                            edge_in = True
                if edge_out:
                    result[y][x] = part.outline or self.outline_key
                elif edge_in:
                    result[y][x] = part.line
        grid = ["".join(r) for r in result]
        for rows, sx, sy in self.stamps:
            grid = paste(grid, rows, sx, sy)
        return grid

    def _shade(self, part, x, y):
        ramp = part.ramp
        if len(ramp) == 1 or part.shade == "flat":
            return ramp[min(1, len(ramp) - 1)]
        if part.shade == "sphere":
            return ramp[_sphere_level(part, x, y, len(ramp))]
        # rim
        up = (x, y - 1) in part.pixels
        down1 = (x, y + 1) in part.pixels
        down2 = (x, y + 2) in part.pixels
        if not up and len(ramp) >= 3:
            return ramp[2]
        if not down1 or not down2:
            return ramp[0]
        return ramp[1]


def figure_bounds(grid):
    xs, ys = [], []
    for y, row in enumerate(grid):
        for x, ch in enumerate(row):
            if ch != TRANSPARENT:
                xs.append(x)
                ys.append(y)
    if not xs:
        return None
    return min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1


def contact_sheet(path, grids, palette, scale=4, cols=6, pad=6, bg=(236, 236, 240, 255), cell=None):
    """Lays grids out left to right, bottom-aligned in equal cells, and writes a PNG."""
    cw = max(grid_size(g)[0] for g in grids) if cell is None else cell[0]
    ch = max(grid_size(g)[1] for g in grids) if cell is None else cell[1]
    rows_n = (len(grids) + cols - 1) // cols
    width = pad + cols * (cw * scale + pad)
    height = pad + rows_n * (ch * scale + pad)
    canvas = rgba_canvas(width, height, bg)
    for n, g in enumerate(grids):
        gw, gh = grid_size(g)
        x = pad + (n % cols) * (cw * scale + pad) + (cw - gw) * scale // 2
        y = pad + (n // cols) * (ch * scale + pad) + (ch - gh) * scale
        rgba_blit(canvas, rgba_scale(grid_to_rgba(g, palette), scale), x, y)
    write_png(path, width, height, [bytes(r) for r in canvas])
