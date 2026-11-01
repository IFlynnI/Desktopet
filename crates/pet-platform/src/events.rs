/// What the shell asks of the engine. It arrives on a channel, followed by a wake-up in case the engine is stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellEvent {
    Toggle,
    Show,
}

/// Wakes the engine. `pet-platform` can't name Bevy's types, so the app passes this in.
pub type Wake = Box<dyn Fn() + Send>;
