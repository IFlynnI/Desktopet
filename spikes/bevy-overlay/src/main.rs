//! THROWAWAY platform spike — see docs/PRD.md §7.1.
//! A transparent, always-on-top, per-pixel click-through Bevy overlay covering the primary
//! monitor's work area, with a walking/draggable placeholder pet, plus instrumentation.

mod hit_mask;
mod sprite;
mod win32;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::settings::{Backends, MemoryHints, WgpuSettings};
use bevy::render::view::Msaa;
use bevy::window::{
    CompositeAlphaMode, CursorOptions, PresentMode, PrimaryWindow, RawHandleWrapper, WindowLevel,
    WindowResolution,
};
use bevy::winit::{UpdateMode, WinitSettings};
use raw_window_handle::RawWindowHandle;

use hit_mask::{AlphaMask, Placement, hits};
use win32::Shared;

struct Args {
    minutes: f64,
    fps: f64,
    scale: u32,
    backend: String,
    present: String,
    alpha: String,
    selftest: bool,
    layered_attrs: bool,
    poll_ms: u64,
    msaa: u32,
    /// Optional smaller window (WxH), anchored bottom-left of the work area.
    win: Option<(u32, u32)>,
    mem_hint: String,
    out: PathBuf,
}

fn parse_args() -> Args {
    let mut a = Args {
        minutes: 60.0,
        fps: 30.0,
        scale: 4,
        backend: "dx12".into(),
        present: "visual".into(),
        alpha: "premultiplied".into(),
        selftest: true,
        layered_attrs: false,
        poll_ms: 8,
        msaa: 1,
        win: None,
        mem_hint: "usage".into(),
        out: PathBuf::from("results/latest"),
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        let val = || argv.get(i + 1).cloned().unwrap_or_default();
        match argv[i].as_str() {
            "--minutes" => { a.minutes = val().parse().expect("--minutes"); i += 1; }
            "--fps" => { a.fps = val().parse().expect("--fps"); i += 1; }
            "--scale" => { a.scale = val().parse().expect("--scale"); i += 1; }
            "--backend" => { a.backend = val(); i += 1; }
            "--present" => { a.present = val(); i += 1; }
            "--alpha" => { a.alpha = val(); i += 1; }
            "--poll-ms" => { a.poll_ms = val().parse().expect("--poll-ms"); i += 1; }
            "--msaa" => { a.msaa = val().parse().expect("--msaa"); i += 1; }
            "--win" => {
                let v = val();
                let (w, h) = v.split_once('x').expect("--win WxH");
                a.win = Some((w.parse().unwrap(), h.parse().unwrap()));
                i += 1;
            }
            "--mem-hint" => { a.mem_hint = val(); i += 1; }
            "--out" => { a.out = PathBuf::from(val()); i += 1; }
            "--no-selftest" => a.selftest = false,
            "--layered-attrs" => a.layered_attrs = true,
            other => panic!("unknown arg {other}"),
        }
        i += 1;
    }
    a
}

#[derive(Resource)]
struct SharedRes(Arc<Shared>);

#[derive(Resource)]
struct Masks(Arc<Vec<AlphaMask>>);

#[derive(Resource)]
struct Pet {
    /// Top-left of the sprite in window pixels.
    pos: Vec2,
    vel: Vec2,
    facing_right: bool,
    frame: usize,
    anim: Timer,
    grab_offset: Option<Vec2>,
    frames: Vec<Handle<Image>>,
    size: Vec2,
    scale: u32,
    window_size: Vec2,
    window_origin: IVec2,
}

impl Pet {
    fn ground_y(&self) -> f32 {
        self.window_size.y - self.size.y
    }

    fn placement(&self) -> Placement {
        Placement {
            left: self.window_origin.x + self.pos.x.round() as i32,
            top: self.window_origin.y + self.pos.y.round() as i32,
            scale: self.scale,
            flip_x: self.facing_right, // art faces left
        }
    }
}

#[derive(Component)]
struct PetSprite;

const WALK_SPEED: f32 = 60.0;
const GRAVITY: f32 = 2400.0;

fn main() {
    let args = parse_args();
    win32::make_dpi_aware();

    // SAFETY: single-threaded at this point; wgpu reads this while creating the instance.
    unsafe {
        if args.present == "visual" {
            std::env::set_var("WGPU_DX12_PRESENTATION_SYSTEM", "Visual");
        } else {
            std::env::set_var("WGPU_DX12_PRESENTATION_SYSTEM", "Hwnd");
        }
    }
    let backends = match args.backend.as_str() {
        "vulkan" => Backends::VULKAN,
        "dx12" => Backends::DX12,
        other => panic!("unknown backend {other}"),
    };
    let alpha = match args.alpha.as_str() {
        "premultiplied" => CompositeAlphaMode::PreMultiplied,
        "postmultiplied" => CompositeAlphaMode::PostMultiplied,
        "auto" => CompositeAlphaMode::Auto,
        other => panic!("unknown alpha {other}"),
    };

    let work = win32::primary_work_area();
    let (win_w, win_h) = args.win.unwrap_or((
        ((work.right - work.left) & !1) as u32,
        ((work.bottom - work.top) & !1) as u32,
    ));
    let origin = IVec2::new(work.left, work.bottom - win_h as i32);

    let art = sprite::creature();
    let masks: Arc<Vec<AlphaMask>> = Arc::new(
        art.rgba.iter().map(|px| AlphaMask::from_rgba(art.width, art.height, px, 128)).collect(),
    );

    let shared = Arc::new(Shared::default());
    shared.report.lock().unwrap().push(format!(
        "config: backend={} present={} alpha={} fps_cap={} scale={}x poll={}ms msaa={} mem_hint={} layered_attrs={} window={}x{} at ({},{})",
        args.backend, args.present, args.alpha, args.fps, args.scale, args.poll_ms, args.msaa, args.mem_hint, args.layered_attrs, win_w, win_h, origin.x, origin.y
    ));

    win32::spawn_hit_thread(shared.clone(), masks.clone(), Duration::from_millis(args.poll_ms), args.layered_attrs);
    win32::spawn_metrics_thread(
        shared.clone(),
        args.out.clone(),
        Duration::from_secs_f64(args.minutes * 60.0),
        Duration::from_secs(10),
        Duration::from_secs(300),
    );
    win32::spawn_startup_shot(shared.clone(), args.out.clone(), Duration::from_secs(8));
    if args.selftest {
        win32::spawn_cursor_selftest(shared.clone(), masks.clone(), Duration::from_secs(12), Duration::from_millis(30));
    }

    let frame_wait = Duration::from_secs_f64(1.0 / args.fps);
    App::new()
        .insert_resource(ClearColor(Color::NONE))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::reactive_low_power(frame_wait),
            unfocused_mode: UpdateMode::reactive_low_power(frame_wait),
        })
        .insert_resource(SharedRes(shared))
        .insert_resource(Masks(masks))
        .insert_resource(ArtRes(art))
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<PipelinedRenderingPlugin>()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "desktop-pet spike".into(),
                        decorations: false,
                        transparent: true,
                        resizable: false,
                        focused: false,
                        composite_alpha_mode: alpha,
                        window_level: WindowLevel::AlwaysOnTop,
                        skip_taskbar: true,
                        position: WindowPosition::At(origin),
                        resolution: WindowResolution::new(win_w, win_h).with_scale_factor_override(1.0),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    primary_cursor_options: Some(CursorOptions { hit_test: false, ..default() }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: WgpuSettings {
                        backends: Some(backends),
                        memory_hints: if args.mem_hint == "performance" {
                            MemoryHints::Performance
                        } else {
                            MemoryHints::MemoryUsage
                        },
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        )
        .insert_resource(SpawnInfo {
            msaa: match args.msaa {
                1 => Msaa::Off,
                2 => Msaa::Sample2,
                4 => Msaa::Sample4,
                n => panic!("unsupported msaa {n}"),
            },
            scale: args.scale,
            window_size: Vec2::new(win_w as f32, win_h as f32),
            window_origin: origin,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (capture_hwnd, drag, simulate, animate, place_sprite, publish).chain())
        .run();
}

#[derive(Resource)]
struct ArtRes(sprite::Frames);

#[derive(Resource)]
struct SpawnInfo {
    msaa: Msaa,
    scale: u32,
    window_size: Vec2,
    window_origin: IVec2,
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, art: Res<ArtRes>, info: Res<SpawnInfo>) {
    commands.spawn((Camera2d, info.msaa));
    let art = &art.0;
    let frames: Vec<Handle<Image>> = art
        .rgba
        .iter()
        .map(|px| {
            let mut image = Image::new(
                Extent3d { width: art.width, height: art.height, depth_or_array_layers: 1 },
                TextureDimension::D2,
                px.clone(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = ImageSampler::nearest();
            images.add(image)
        })
        .collect();
    let size = Vec2::new((art.width * info.scale) as f32, (art.height * info.scale) as f32);
    let pet = Pet {
        pos: Vec2::new(info.window_size.x * 0.5, info.window_size.y - size.y),
        vel: Vec2::new(WALK_SPEED, 0.0),
        facing_right: true,
        frame: 0,
        anim: Timer::from_seconds(0.25, TimerMode::Repeating),
        grab_offset: None,
        frames: frames.clone(),
        size,
        scale: info.scale,
        window_size: info.window_size,
        window_origin: info.window_origin,
    };
    commands.spawn((
        PetSprite,
        Sprite { image: frames[0].clone(), custom_size: Some(size), ..default() },
        Transform::default(),
    ));
    commands.insert_resource(pet);
}

fn capture_hwnd(shared: Res<SharedRes>, windows: Query<&RawHandleWrapper, With<PrimaryWindow>>) {
    if shared.0.hwnd.load(Ordering::Acquire) != 0 {
        return;
    }
    if let Ok(handle) = windows.single()
        && let RawWindowHandle::Win32(h) = handle.get_window_handle()
    {
        shared.0.hwnd.store(h.hwnd.get(), Ordering::Release);
    }
}

fn drag(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    masks: Res<Masks>,
    shared: Res<SharedRes>,
    mut pet: ResMut<Pet>,
) {
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    if buttons.just_pressed(MouseButton::Left) {
        let local = Placement { left: pet.pos.x.round() as i32, top: pet.pos.y.round() as i32, ..pet.placement() };
        let mask = &masks.0[pet.frame % masks.0.len()];
        if hits(mask, &local, (cursor.x as i32, cursor.y as i32)) {
            pet.grab_offset = Some(cursor - pet.pos);
            shared.0.dragging.store(true, Ordering::Release);
        }
    }
    if let Some(offset) = pet.grab_offset {
        if buttons.pressed(MouseButton::Left) {
            let max = pet.window_size - pet.size;
            pet.pos = (cursor - offset).clamp(Vec2::ZERO, max);
            pet.vel = Vec2::ZERO;
        } else {
            pet.grab_offset = None;
            shared.0.dragging.store(false, Ordering::Release);
        }
    }
}

fn simulate(time: Res<Time>, shared: Res<SharedRes>, mut pet: ResMut<Pet>) {
    if pet.grab_offset.is_some() || shared.0.frozen.load(Ordering::Acquire) {
        return;
    }
    let dt = time.delta_secs().min(0.1);
    let ground = pet.ground_y();
    if pet.pos.y < ground {
        pet.vel.y += GRAVITY * dt;
        pet.pos.y = (pet.pos.y + pet.vel.y * dt).min(ground);
        if pet.pos.y >= ground {
            pet.vel = Vec2::new(if pet.facing_right { WALK_SPEED } else { -WALK_SPEED }, 0.0);
        }
        return;
    }
    pet.pos.x += pet.vel.x * dt;
    let max_x = pet.window_size.x - pet.size.x;
    if pet.pos.x <= 0.0 || pet.pos.x >= max_x {
        pet.pos.x = pet.pos.x.clamp(0.0, max_x);
        pet.vel.x = -pet.vel.x;
    }
    pet.facing_right = pet.vel.x > 0.0;
}

fn animate(time: Res<Time>, shared: Res<SharedRes>, mut pet: ResMut<Pet>) {
    if shared.0.frozen.load(Ordering::Acquire) {
        return;
    }
    pet.anim.tick(time.delta());
    if pet.anim.just_finished() {
        pet.frame = (pet.frame + 1) % pet.frames.len();
    }
}

fn place_sprite(pet: Res<Pet>, mut q: Query<(&mut Sprite, &mut Transform), With<PetSprite>>) {
    let Ok((mut sprite, mut transform)) = q.single_mut() else { return };
    let pos = pet.pos.round();
    transform.translation = Vec3::new(
        pos.x + pet.size.x / 2.0 - pet.window_size.x / 2.0,
        pet.window_size.y / 2.0 - (pos.y + pet.size.y / 2.0),
        0.0,
    );
    sprite.flip_x = pet.facing_right;
    if sprite.image != pet.frames[pet.frame] {
        sprite.image = pet.frames[pet.frame].clone();
    }
}

/// Hands the drawn placement to the hit-test thread and counts frames.
fn publish(pet: Res<Pet>, shared: Res<SharedRes>) {
    *shared.0.placement.lock().unwrap() = pet.placement();
    shared.0.frame.store(pet.frame, Ordering::Release);
    shared.0.frames_rendered.fetch_add(1, Ordering::Relaxed);
}
