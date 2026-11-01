//! The operating-system boundary. Everything here is OS-neutral; each OS gets its own folder.

mod events;
mod hit_target;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
use self::windows as os;

#[cfg(not(windows))]
compile_error!("Desktop Pet only supports Windows so far (PRD §3)");

pub use events::{ShellEvent, Wake};
pub use hit_target::HitTarget;
pub use os::{Platform, home_work_area, prepare_process};
