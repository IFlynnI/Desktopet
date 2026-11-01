//! Desktop Pet: wires the engine to the platform.

// Release builds run without a console window. Debug builds keep it for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;

use bevy::app::AppExit;

fn main() -> AppExit {
    pet_platform::prepare_process();
    engine::app(pet_platform::home_work_area()).run()
}
