//! Starts a fresh process after a render failure. The new process gets the Relaunch history and the Manual hide as
//! arguments, because nothing else survives the old one.

use std::env;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::log::error;
use pet_core::relaunch::{MAX_RELAUNCHES, record_relaunch};

const HISTORY_ARG: &str = "--relaunch-history=";
const MANUAL_HIDE_ARG: &str = "--manual-hide";

/// What this process was started with. A normal launch has neither.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Launch {
    /// When earlier Relaunches happened, in seconds.
    pub history: Vec<u64>,
    pub manual_hide: bool,
}

impl Launch {
    pub fn from_args(args: impl Iterator<Item = String>) -> Self {
        let mut launch = Self::default();
        for arg in args {
            if arg == MANUAL_HIDE_ARG {
                launch.manual_hide = true;
            } else if let Some(times) = arg.strip_prefix(HISTORY_ARG) {
                let times = times.split(',').filter_map(|time| time.parse().ok());
                launch.history = times.collect();
            }
        }
        launch
    }

    fn to_args(&self) -> Vec<String> {
        let times: Vec<String> = self.history.iter().map(u64::to_string).collect();
        let mut args = vec![format!("{HISTORY_ARG}{}", times.join(","))];
        if self.manual_hide {
            args.push(MANUAL_HIDE_ARG.to_owned());
        }
        args
    }
}

/// Call this only after the mutex is released, or the new process finds the old Pet still here and exits.
pub fn relaunch(history: &[u64], manual_hide: bool) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs());
    let Some(history) = record_relaunch(history, now) else {
        error!(
            "rendering failed after {MAX_RELAUNCHES} Relaunches in 10 minutes, so the app stays closed"
        );
        return;
    };
    let next = Launch {
        history,
        manual_hide,
    };
    let started = env::current_exe().and_then(|exe| Command::new(exe).args(next.to_args()).spawn());
    if let Err(error) = started {
        error!("couldn't relaunch after a render failure: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(args: &[&str]) -> impl Iterator<Item = String> {
        args.iter()
            .map(|arg| arg.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn given_no_arguments_when_reading_them_then_it_is_a_normal_launch() {
        // Given / When
        let launch = Launch::from_args(args(&[]));

        // Then
        assert_eq!(launch, Launch::default());
    }

    #[test]
    fn given_a_relaunch_when_passing_its_details_on_then_the_new_process_reads_the_same_values() {
        // Given
        let launch = Launch {
            history: vec![1000, 2000],
            manual_hide: true,
        };

        // When
        let read = Launch::from_args(launch.to_args().into_iter());

        // Then
        assert_eq!(read, launch);
    }

    #[test]
    fn given_no_manual_hide_when_passing_details_on_then_the_new_process_shows_the_pet() {
        // Given
        let launch = Launch {
            history: vec![1000],
            manual_hide: false,
        };

        // When
        let read = Launch::from_args(launch.to_args().into_iter());

        // Then
        assert!(!read.manual_hide);
    }

    #[test]
    fn given_arguments_it_does_not_know_when_reading_them_then_they_are_ignored() {
        // Given
        let given = args(&["--something-else", "--relaunch-history=12,oops,34"]);

        // When
        let launch = Launch::from_args(given);

        // Then: a bad time is dropped, not fatal
        assert_eq!(launch.history, vec![12, 34]);
        assert!(!launch.manual_hide);
    }
}
