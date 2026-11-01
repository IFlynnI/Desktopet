//! Tests against the real exe on the real desktop. `cargo e2e-quiet` runs the ones that leave the mouse alone, and
//! `cargo e2e` runs everything.
#![cfg(windows)]

mod hands_off;
mod harness;
mod quiet;
mod screen;
mod test_window;
