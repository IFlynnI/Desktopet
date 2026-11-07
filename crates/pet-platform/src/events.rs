use pet_core::geometry::Rect;

/// What the shell asks of the engine. It arrives on a channel, followed by a wake-up in case the engine is stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellEvent {
    Toggle,
    Show,
    Quit,
    /// Whether a fullscreen application is now in the foreground. It's sent only when that changes.
    FullscreenChanged(bool),
    /// The Home monitor's new work area, the part the taskbar leaves free. It's sent only when that changes.
    WorkAreaChanged(Rect),
}

/// The tray icon as plain pixels, so this crate stays free of art.
#[derive(Clone, Debug)]
pub struct TrayIconImage {
    /// RGBA8, top row first.
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Wakes the engine. `pet-platform` can't name Bevy's types, so the app passes this in.
pub type Wake = Box<dyn Fn() + Send>;
