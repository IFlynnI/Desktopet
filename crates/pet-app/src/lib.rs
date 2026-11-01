//! Desktop Pet: wires the engine to the platform.

mod engine;
mod relaunch;

pub use engine::RENDER_FAILURE_EXIT;

use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::app::{App, AppExit};
use pet_core::presence::Presence;
use relaunch::Launch;

/// Runs the app until it exits. `setup` sees the finished `App` first, so another program can add to it.
pub fn run(setup: impl FnOnce(&mut App)) -> AppExit {
    pet_platform::prepare_process();
    let args = env::args_os().skip(1);
    let launch = Launch::from_args(args.map(|arg| arg.to_string_lossy().into_owned()));
    let Some(first_pet) = pet_platform::acquire_single_instance() else {
        return AppExit::Success;
    };

    let manual_hide = Arc::new(AtomicBool::new(launch.manual_hide));
    let start = Presence::with_manual_hide(launch.manual_hide);
    let mut app = engine::app(pet_platform::home_work_area(), start, manual_hide.clone());
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
