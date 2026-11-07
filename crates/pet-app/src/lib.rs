//! Desktop Pet: wires the engine to the platform.

mod config;
mod engine;
mod pack;
mod relaunch;

pub use engine::{
    PackAtStartup, PackEdited, PetRenderPlugin, RENDER_FAILURE_EXIT, base_plugins, pet_camera,
    render_plugin,
};
pub use pack::StartingPack;

use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::app::{App, AppExit};
use pet_core::presence::Presence;
use relaunch::Launch;

use pack::PACKS_OVERRIDE;

/// Runs the app until it exits. `setup` sees the finished `App` first, so another program can add to it.
pub fn run(setup: impl FnOnce(&mut App)) -> AppExit {
    pet_platform::prepare_process();
    let args = env::args_os().skip(1);
    let launch = Launch::from_args(args.map(|arg| arg.to_string_lossy().into_owned()));
    let Some(first_pet) = pet_platform::acquire_single_instance() else {
        return AppExit::Success;
    };

    // Only the first Pet reads the Pack. A second launch exits above, without touching the Packs or the log.
    let packs_folder =
        pack::packs_folder(env::var_os(PACKS_OVERRIDE), pet_platform::packs_folder());
    let chosen = packs_folder.as_deref().and_then(pet_content::discover);
    // Watched before it's read, so an edit made while the app starts is still seen.
    let watching = engine::Watching::start(chosen.as_deref());
    let pack = StartingPack::load_chosen(packs_folder, chosen);

    let manual_hide = Arc::new(AtomicBool::new(launch.manual_hide));
    let start = Presence::with_manual_hide(launch.manual_hide);
    let work_area = pet_platform::home_work_area();
    let home_monitor = pet_platform::home_monitor().unwrap_or(work_area);
    let config_folder = pet_platform::config_folder();
    let config = config::StartingConfig::load(config_folder.as_deref(), home_monitor);
    let mut app = engine::app(
        work_area,
        start,
        manual_hide.clone(),
        pack,
        watching,
        config,
    );
    setup(&mut app);
    let exit = app.run();

    // The world has gone by now, and with it `Platform`, so the tray icon and hotkey are free. The mutex goes next: a
    // new process that found it held would think the old Pet was still here, and exit.
    drop(first_pet);
    if engine::is_render_failure(&exit) {
        relaunch::relaunch(&launch.history, manual_hide.load(Ordering::Relaxed));
    }
    exit
}
