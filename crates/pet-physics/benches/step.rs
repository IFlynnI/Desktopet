//! What a tick of physics costs: one Pet, and 1,000 Bodies flying among a few Obstacles, as a later track's enemies
//! would. Std only, so it needs no benchmark crate. The research measured about 0.2 µs and 190 µs
//! (docs/research/physics-rotation.md).

use std::hint::black_box;
use std::time::Instant;

use pet_physics::geometry::Rect;
use pet_physics::{Body, Extent, Material, Obstacle, Vec2, World, release, step};

const DT: f32 = 1.0 / 30.0;

const SURFACE: Material = Material {
    friction: 0.8,
    bounciness: 0.3,
};

fn world() -> World {
    let ledge = |left, top, right, bottom| Obstacle {
        rect: Rect {
            left,
            top,
            right,
            bottom,
        },
        material: SURFACE,
    };
    World {
        bounds: Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        },
        surface: SURFACE,
        gravity: 3000.0,
        obstacles: vec![
            ledge(300, 700, 700, 716),
            ledge(900, 500, 1300, 516),
            ledge(1500, 300, 1540, 1040),
            ledge(100, 300, 400, 316),
        ],
    }
}

/// A small seeded generator, so every run throws the same Bodies.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

fn thrown(random: &mut Lcg) -> Body {
    let mut body = Body::new(
        Vec2::new(
            100.0 + random.next() * 1700.0,
            100.0 + random.next() * 900.0,
        ),
        Extent::scaled(-16, -30, 32, 30, 2),
        SURFACE,
    );
    body.velocity = Vec2::new(random.next() - 0.5, random.next() - 0.5) * 6000.0;
    release(&mut body, 2500.0);
    body
}

/// Steps `bodies` for `ticks` and gives the microseconds per tick.
fn time(world: &World, bodies: &mut [Body], ticks: u32) -> f64 {
    let start = Instant::now();
    for _ in 0..ticks {
        for body in bodies.iter_mut() {
            black_box(step(black_box(world), body, None, DT));
        }
    }
    start.elapsed().as_secs_f64() * 1e6 / f64::from(ticks)
}

fn main() {
    let world = world();
    let mut random = Lcg(7);

    let mut one = [thrown(&mut random)];
    let mut one_total = 0.0;
    let runs = 100;
    for _ in 0..runs {
        one[0] = thrown(&mut random);
        one_total += time(&world, &mut one, 300);
    }
    println!("1 Body: {:.3} µs per tick", one_total / f64::from(runs));

    let mut many: Vec<Body> = (0..1000).map(|_| thrown(&mut random)).collect();
    println!(
        "1,000 Bodies: {:.1} µs per tick",
        time(&world, &mut many, 300)
    );
}
