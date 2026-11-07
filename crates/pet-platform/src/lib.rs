//! The operating-system boundary. Everything here is OS-neutral; each OS gets its own folder.

mod events;
mod hit_target;
mod pointer;
#[cfg(windows)]
mod windows;

#[cfg(windows)]
use self::windows as os;

#[cfg(not(windows))]
compile_error!("Desktop Pet only supports Windows so far (PRD §3)");

pub use events::{ShellEvent, TrayIconImage, Wake};
pub use hit_target::HitTarget;
pub use os::{
    FolderWatch, Platform, SingleInstance, acquire_single_instance, config_folder, home_monitor,
    home_work_area, log_file_path, packs_folder, prepare_process,
};
pub use pointer::PointerSample;
