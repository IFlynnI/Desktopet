//! The Desktop Pet program. It's all in the library, so a test program can run the same app.

// Release builds run without a console window. Debug builds keep it for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::app::AppExit;

fn main() -> AppExit {
    pet_app::run(|_app| {})
}
