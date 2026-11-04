"""A tiny 3D model of Brim, drawn as pixel art.

Why 3D: the parts of a character overlap in depth. A flat rig can't make the helmet's rim change
shape as it rocks, show the shell's inside when it tips back, or keep the far horn and far eye in
the right place. Here every part is a real 3D shape seen by one fixed camera, so perspective and
overlaps are always consistent. Each pixel casts one ray; the first shape it hits gives the pixel
its part and its shade. Then the same pixel-art rules as before apply: flat cel bands, an outer
outline, coloured lines inside, hand-drawn stamps for the face, and a paint-over on top.

The camera: orthographic, Brim turned towards the viewer by YAW degrees and seen from a little above
(PITCH). The light is fixed to the camera, so a mirrored Frame is still lit from above.

Units are cell pixels. Model space: +x is where Brim faces, +y is up, +z is towards Brim's near
side (the viewer's side). The origin is the Anchor, on the ground between the feet.
"""

import math

YAW = 28.0      # degrees Brim is turned towards the viewer
PITCH = 12.0    # degrees the camera looks down
LIGHT = (0.0, 0.78, 0.62)   # camera space, towards the light: above and in front


# ------------------------------------------------------------------ small linear algebra

def norm(v):
    n = math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) or 1.0
    return (v[0] / n, v[1] / n, v[2] / n)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def add(a, b):
    return (a[0] + b[0], a[1] + b[1], a[2] + b[2])


def mul(a, s):
    return (a[0] * s, a[1] * s, a[2] * s)


def lerp(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t)


class Affine:
    """p -> M p + t. Compose with `a @ b` meaning "apply b, then a"."""

    def __init__(self, m=None, t=(0.0, 0.0, 0.0)):
        self.m = m or ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
        self.t = t

    def __matmul__(self, o):
        a, b = self.m, o.m
        m = tuple(tuple(sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)) for i in range(3))
        return Affine(m, add(self.vec(o.t), self.t))

    def vec(self, v):
        m = self.m
        return (m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
                m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
                m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2])

    def point(self, p):
        return add(self.vec(p), self.t)

    def inverse(self):
        m = self.m
        det = (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
               - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
               + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))
        inv = (
            ((m[1][1] * m[2][2] - m[1][2] * m[2][1]) / det, (m[0][2] * m[2][1] - m[0][1] * m[2][2]) / det,
             (m[0][1] * m[1][2] - m[0][2] * m[1][1]) / det),
            ((m[1][2] * m[2][0] - m[1][0] * m[2][2]) / det, (m[0][0] * m[2][2] - m[0][2] * m[2][0]) / det,
             (m[0][2] * m[1][0] - m[0][0] * m[1][2]) / det),
            ((m[1][0] * m[2][1] - m[1][1] * m[2][0]) / det, (m[0][1] * m[2][0] - m[0][0] * m[2][1]) / det,
             (m[0][0] * m[1][1] - m[0][1] * m[1][0]) / det),
        )
        a = Affine(inv)
        return Affine(inv, mul(a.vec(self.t), -1.0))

    def normal(self, n):
        """Transforms a surface normal (by the inverse transpose)."""
        inv = self.inverse().m
        return norm((inv[0][0] * n[0] + inv[1][0] * n[1] + inv[2][0] * n[2],
                     inv[0][1] * n[0] + inv[1][1] * n[1] + inv[2][1] * n[2],
                     inv[0][2] * n[0] + inv[1][2] * n[1] + inv[2][2] * n[2]))


def translate(x, y, z):
    return Affine(t=(x, y, z))


def scale(sx, sy, sz):
    return Affine(((sx, 0.0, 0.0), (0.0, sy, 0.0), (0.0, 0.0, sz)))


def rot_x(deg):
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    return Affine(((1.0, 0.0, 0.0), (0.0, c, -s), (0.0, s, c)))


def rot_y(deg):
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    return Affine(((c, 0.0, s), (0.0, 1.0, 0.0), (-s, 0.0, c)))


def rot_z(deg):
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    return Affine(((c, -s, 0.0), (s, c, 0.0), (0.0, 0.0, 1.0)))


def rot_axis(axis, deg):
    """Rotation by `deg` about a unit `axis` through the origin (Rodrigues)."""
    x, y, z = norm(axis)
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    k = 1 - c
    return Affine(((c + x * x * k, x * y * k - z * s, x * z * k + y * s),
                   (y * x * k + z * s, c + y * y * k, y * z * k - x * s),
                   (z * x * k - y * s, z * y * k + x * s, c + z * z * k)))


def about(pivot, xf):
    """`xf` applied about `pivot` instead of the origin."""
    return translate(*pivot) @ xf @ translate(-pivot[0], -pivot[1], -pivot[2])


def camera(anchor):
    """Model space to camera space: turn towards the viewer, tilt for the view from above, then
    put the model origin on the Anchor. Camera space: x right, y up (cell rows go down), z towards
    the viewer."""
    ax, ay = anchor
    return translate(ax, -ay, 0.0) @ rot_x(PITCH) @ rot_y(-YAW)


# ------------------------------------------------------------------ shapes in their own local space

def _ellipsoid_hits(o, d, r):
    """Ray o + t d against the ellipsoid with radii r at the origin. Returns (t0, t1) or None."""
    ox, oy, oz = o[0] / r[0], o[1] / r[1], o[2] / r[2]
    dx, dy, dz = d[0] / r[0], d[1] / r[1], d[2] / r[2]
    a = dx * dx + dy * dy + dz * dz
    b = 2 * (ox * dx + oy * dy + oz * dz)
    c = ox * ox + oy * oy + oz * oz - 1
    disc = b * b - 4 * a * c
    if disc < 0:
        return None
    s = math.sqrt(disc)
    return (-b - s) / (2 * a), (-b + s) / (2 * a)


class Ellipsoid:
    def __init__(self, radii):
        self.r = radii

    def bound(self):
        return max(self.r)

    def hit(self, o, d):
        h = _ellipsoid_hits(o, d, self.r)
        if h is None or h[1] < 0:
            return None
        t = h[0]
        p = add(o, mul(d, t))
        n = norm((p[0] / self.r[0] ** 2, p[1] / self.r[1] ** 2, p[2] / self.r[2] ** 2))
        return t, n, "outer"


def _ellipsoid_field(p, r):
    q = (p[0] / r[0], p[1] / r[1], p[2] / r[2])
    f = math.sqrt(q[0] * q[0] + q[1] * q[1] + q[2] * q[2]) - 1.0
    return f * min(r), (p[0] / r[0] ** 2, p[1] / r[1] ** 2, p[2] / r[2] ** 2)


Ellipsoid.field = lambda self, p: _ellipsoid_field(p, self.r)


class Tube:
    """A tapering tube along a path, as overlapping spheres; looks smooth at pixel size."""

    def __init__(self, path, radii, density=2.5):
        self.spheres = []
        for (p1, p2), (r1, r2) in zip(zip(path, path[1:]), zip(radii, radii[1:])):
            n = max(1, int(math.dist(p1, p2) * density / max(min(r1, r2), 0.6)))
            for k in range(n + (1 if p2 is path[-1] else 0)):
                t = k / n
                self.spheres.append((lerp(p1, p2, t), r1 + (r2 - r1) * t))
        self._bound = max(math.dist((0, 0, 0), c) + r for c, r in self.spheres)

    def bound(self):
        return self._bound

    def field(self, p):
        best = None
        for c, r in self.spheres:
            v = sub(p, c)
            dist = math.sqrt(dot(v, v)) - r
            if best is None or dist < best[0]:
                best = (dist, v)
        return best

    def hit(self, o, d):
        best = None
        for c, r in self.spheres:
            oc = sub(o, c)
            b = dot(oc, d)
            cc = dot(oc, oc) - r * r
            disc = b * b - dot(d, d) * cc
            if disc < 0:
                continue
            t = (-b - math.sqrt(disc)) / dot(d, d)
            if best is None or t < best[0]:
                best = (t, c)
        if best is None:
            return None
        t, c = best
        return t, norm(sub(add(o, mul(d, t)), c)), "outer"


class Plate:
    """A flat shape in the local x-y plane (z = 0), seen from both sides: a wing membrane."""

    def __init__(self, polygon):
        self.poly = polygon
        self._bound = max(math.hypot(x, y) for x, y in polygon) + 0.5

    def bound(self):
        return self._bound

    def hit(self, o, d):
        if abs(d[2]) < 1e-6:
            return None
        t = -o[2] / d[2]
        if t < 0:
            return None
        x, y = o[0] + d[0] * t, o[1] + d[1] * t
        if not _inside_poly(self.poly, x, y):
            return None
        n = (0.0, 0.0, 1.0) if d[2] < 0 else (0.0, 0.0, -1.0)
        return t, n, "outer"


def _inside_poly(poly, x, y):
    inside, n = False, len(poly)
    for i in range(n):
        x1, y1 = poly[i]
        x2, y2 = poly[(i + 1) % n]
        if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1) + x1:
            inside = not inside
    return inside


class Shell:
    """A piece of eggshell: the solid between two egg surfaces, cut along a crack line.

    The egg is two half-ellipsoids: taller on top (`top`) than below (`bottom`), radius `r` around.
    keep(y, angle) says which side of the crack is kept; angle is around the vertical axis, 0 where
    Brim faces. Holes are (point, direction, radius) lines the shell is pierced by (the horns).
    """

    def __init__(self, r, top, bottom, thickness, crack, keep_top=True, holes=(), holes_xf=None, angles=None):
        self.r, self.top, self.bottom, self.th = r, top, bottom, thickness
        self.crack, self.keep_top, self.holes = crack, keep_top, holes
        # keep only the piece between two angles round the vertical axis: a shard
        self.angles = angles
        # Holes are given in another space (the head's), through `holes_xf` (shell -> that space), so a
        # hole always sits where the horn is, however the shell rocks: the shell has slots, not holes.
        self.holes_xf = holes_xf

    def bound(self):
        return max(self.r, self.top, self.bottom) + 0.5

    def _inside(self, p, shrink):
        h = (self.top if p[1] >= 0 else self.bottom) - shrink
        r = self.r - shrink
        return (p[0] / r) ** 2 + (p[1] / h) ** 2 + (p[2] / r) ** 2 <= 1.0

    def crack_y(self, p):
        return self.crack(math.degrees(math.atan2(p[2], p[0])))

    def _kept(self, p):
        if self.angles is not None:
            a = math.degrees(math.atan2(p[2], p[0]))
            a0, a1 = self.angles
            if not ((a - a0) % 360.0) <= ((a1 - a0) % 360.0):
                return False
        if self.keep_top is None:
            return True
        above = p[1] >= self.crack_y(p)
        return above if self.keep_top else not above

    def _in_hole(self, p):
        if self.holes_xf is not None:
            p = self.holes_xf.point(p)
        for c, direction, rad in self.holes:
            v = sub(p, c)
            along = dot(v, direction)
            perp = sub(v, mul(direction, along))
            if dot(perp, perp) <= rad * rad:
                return True
        return False

    def solid(self, p):
        return (self._inside(p, 0.0) and not self._inside(p, self.th) and self._kept(p)
                and not self._in_hole(p))

    def hit(self, o, d):
        if self.keep_top is None and not self.holes and self.angles is None:
            return self._hit_closed(o, d)
        h = _ellipsoid_hits(o, d, (self.r, max(self.top, self.bottom), self.r))
        if h is None or h[1] < 0:
            return None
        t0, t1 = max(h[0], 0.0), h[1]
        step = 0.3
        t = t0
        prev = None
        while t <= t1 + step:
            p = add(o, mul(d, t))
            if self.solid(p):
                lo, hi = (prev if prev is not None else t - step), t
                for _ in range(8):
                    mid = (lo + hi) / 2
                    if self.solid(add(o, mul(d, mid))):
                        hi = mid
                    else:
                        lo = mid
                p = add(o, mul(d, hi))
                return hi, self._normal(p), self._face(add(o, mul(d, lo)))
            prev = t
            t += step
        return None

    def _hit_closed(self, o, d):
        best = None
        for h, upper in ((self.top, True), (self.bottom, False)):
            hh = _ellipsoid_hits(o, d, (self.r, h, self.r))
            if hh is None or hh[1] < 0:
                continue
            t = hh[0]
            p = add(o, mul(d, t))
            if (p[1] >= 0) == upper and (best is None or t < best[0]):
                best = (t, self._normal(p), "outer")
        return best

    def _face(self, outside_point):
        """Which surface the ray came through, judged from the empty point just before the hit."""
        p = outside_point
        if not self._inside(p, 0.0):
            return "outer"
        if self._in_hole(p):
            return "hole"
        if self._inside(p, self.th):
            return "inner"
        return "rim"

    def _normal(self, p):
        h = self.top if p[1] >= 0 else self.bottom
        return norm((p[0] / self.r ** 2, p[1] / h ** 2, p[2] / self.r ** 2))


# ------------------------------------------------------------------ parts and the scene

class Part:
    """A shape placed in model space by `xf` (local -> model)."""

    def __init__(self, name, shape, xf, ramp, group, line, bands=None, inner_ramp=None, casts=False):
        self.name, self.shape, self.xf = name, shape, xf
        self.ramp, self.group, self.line, self.bands = ramp, group, line, bands
        self.inner_ramp = inner_ramp
        self.casts = casts


def render(parts, width, height, anchor, shadow_from=(), shadow_on=(), shadow_reach=3.0, smooth=(), blend=1.6):
    """Casts one ray per pixel. Returns a dict {(x, y): hit} where a hit is
    (part, t, normal in camera space, face)."""
    cam = camera(anchor)
    placed = []
    for part in parts:
        to_cam = cam @ part.xf
        to_local = to_cam.inverse()
        centre = to_cam.point((0.0, 0.0, 0.0))
        # a generous screen-space bound from the shape's bounding sphere and the transform's stretch
        stretch = max(math.sqrt(sum(to_cam.m[i][j] ** 2 for i in range(3))) for j in range(3))
        rad = part.shape.bound() * stretch + 1.5
        placed.append((part, to_cam, to_local, centre, rad))
    hits = {}
    d_cam = (0.0, 0.0, -1.0)
    for y in range(height):
        cy = -(y + 0.5)
        for x in range(width):
            cx = x + 0.5
            best = None
            for part, to_cam, to_local, c, rad in placed:
                if abs(cx - c[0]) > rad or abs(cy - c[1]) > rad:
                    continue
                o = to_local.point((cx, cy, 1000.0))
                d = to_local.vec(d_cam)
                h = part.shape.hit(o, d)
                if h is None:
                    continue
                t, n, face = h
                if best is None or t < best[1]:
                    best = (part, t, to_cam.normal(n), face, add(o, mul(d, t)))
            if best is not None:
                hits[(x, y)] = best
    if smooth:
        _smooth_normals(hits, placed, smooth, blend)
    if shadow_from:
        _cast_shadows(hits, placed, shadow_from, shadow_on, shadow_reach)
    return hits


def _smooth_normals(hits, placed, groups, k):
    """Blends the normals of the parts of one group near where they meet, as if they were one soft
    surface: a thigh flows into the belly and the snout into the head, with no shading seam."""
    members = {}
    for part, to_cam, to_local, c, r in placed:
        if part.group in groups and hasattr(part.shape, "field"):
            members.setdefault(part.group, []).append((part, to_cam, to_local))
    for key, hit in hits.items():
        part, t, n, face, local_p = hit[:5]
        if part.group not in members:
            continue
        x, y = key
        p_cam = (x + 0.5, -(y + 0.5), 1000.0 - t)
        acc = (0.0, 0.0, 0.0)
        for other, to_cam, to_local in members[part.group]:
            dist, grad = other.shape.field(to_local.point(p_cam))
            if dist > 4 * k:
                continue
            w = math.exp(-max(dist, 0.0) / k)
            acc = add(acc, mul(to_cam.normal(grad), w))
        if acc != (0.0, 0.0, 0.0):
            hits[key] = (part, t, norm(acc), face, local_p) + tuple(hit[5:])


def _cast_shadows(hits, placed, casters, receivers, reach):
    """Marks receiver pixels whose surface point can't see the light because a caster is in the way,
    within `reach` pixels: a short contact shadow, like the band under a hat's brim."""
    cast = [(part, to_cam, to_local) for part, to_cam, to_local, c, r in placed if part.name in casters]
    for key, (part, t, n, face, local_p) in list(hits.items()):
        if part.name not in receivers:
            continue
        x, y = key
        p_cam = (x + 0.5, -(y + 0.5), 1000.0 - t)
        for cpart, c_cam, c_local in cast:
            o = c_local.point(add(p_cam, mul(LIGHT, 0.6)))
            d = c_local.vec(LIGHT)
            h = cpart.shape.hit(o, d)
            if h is not None and 0 < h[0] <= reach:
                hits[key] = (part, t, n, face, local_p, "shadow")
                break


def level(n, bands, shadow=False):
    i = dot(n, norm(LIGHT))
    k = 0
    for cut in bands:
        if i >= cut:
            k += 1
    if shadow:
        k = max(0, k - 2)
    return k


def surface_point(radii, direction):
    """The point on an ellipsoid (at the origin) in `direction` from its centre."""
    d = norm(direction)
    k = 1.0 / math.sqrt((d[0] / radii[0]) ** 2 + (d[1] / radii[1]) ** 2 + (d[2] / radii[2]) ** 2)
    return mul(d, k)
