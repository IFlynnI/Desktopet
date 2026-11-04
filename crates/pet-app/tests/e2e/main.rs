//! Tests against the real exe on the real desktop. A running exe always shows something, so the owner starts these,
//! never a background run. `cargo e2e-quiet` runs the ones that send no input, and `cargo e2e` runs everything.
#![cfg(windows)]

mod hands_off;
mod harness;
mod metrics;
mod quiet;
mod screen;
mod test_window;
