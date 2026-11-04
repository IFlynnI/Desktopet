//! Desktop Pet: wires the engine to the platform.

mod engine;
mod pack;
mod relaunch;

pub use engine::RENDER_FAILURE_EXIT;

use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::app::{App, AppExit};
use pet_core::presence::Presence;
use relaunch::Launch;

use pack::{PACKS_OVERRIDE, StartingPack};

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
    let pack = StartingPack::load(packs_folder);

    let manual_hide = Arc::new(AtomicBool::new(launch.manual_hide));
    let start = Presence::with_manual_hide(launch.manual_hide);
    let work_area = pet_platform::home_work_area();
    let mut app = engine::app(work_area, start, manual_hide.clone(), pack);
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
