//! Desktop Pet: wires the engine to the platform.

mod engine;

use bevy::app::{App, AppExit};

/// Runs the app until it exits. `setup` sees the finished `App` first, so another program can add to it.
pub fn run(setup: impl FnOnce(&mut App)) -> AppExit {
    pet_platform::prepare_process();
    let mut app = engine::app(pet_platform::home_work_area());
    setup(&mut app);
    app.run()
}
