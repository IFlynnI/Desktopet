//! Turns a render failure into an exit `pet_app::run` can recognise. Bevy's default handler exits with a generic error,
//! so nothing could tell this failure from any other, and only this one should lead to a Relaunch.

use bevy::app::AppExit;
use bevy::render::error_handler::{RenderErrorHandler, RenderErrorPolicy};

pub const RENDER_FAILURE_EXIT: u8 = 70;

fn exit() -> AppExit {
    AppExit::from_code(RENDER_FAILURE_EXIT)
}

pub fn is_render_failure(exit_code: &AppExit) -> bool {
    *exit_code == exit()
}

/// Bevy logs the error before calling this, and that reaches the log file.
pub fn handler() -> RenderErrorHandler {
    RenderErrorHandler(|_error, main_world, _render_world| {
        main_world.write_message(exit());
        RenderErrorPolicy::StopRendering
    })
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::Messages;
    use bevy::ecs::world::World;
    use bevy::render::error_handler::{ErrorType, RenderError};

    use super::*;

    #[test]
    fn given_a_render_error_when_handling_it_then_the_app_exits_with_the_render_failure_code() {
        // Given
        let (mut main_world, mut render_world) = (World::new(), World::new());
        main_world.init_resource::<Messages<AppExit>>();
        let error = RenderError {
            ty: ErrorType::DeviceLost,
            description: String::new(),
            source: None,
        };

        // When
        let policy = (handler().0)(&error, &mut main_world, &mut render_world);

        // Then
        assert!(matches!(policy, RenderErrorPolicy::StopRendering));
        let messages = main_world.resource::<Messages<AppExit>>();
        let written = messages.get_cursor().read(messages).next().cloned();
        assert_eq!(written, Some(exit()));
        assert!(is_render_failure(&written.unwrap()));
    }

    #[test]
    fn given_any_other_exit_when_checking_it_then_it_is_not_a_render_failure() {
        // Given
        let others = [AppExit::Success, AppExit::error()];

        // When
        let failures = others.iter().filter(|other| is_render_failure(other));

        // Then
        assert_eq!(failures.count(), 0);
    }
}
