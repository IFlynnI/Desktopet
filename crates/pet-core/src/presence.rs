//! Whether the Pet is shown or Hidden, and which hides currently apply.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Presence {
    manual_hide: bool,
    auto_hide: bool,
}

impl Presence {
    /// A normal launch has no Manual hide, and a Relaunch keeps the one it had.
    pub fn with_manual_hide(manual_hide: bool) -> Self {
        Self {
            manual_hide,
            auto_hide: false,
        }
    }

    pub fn is_hidden(self) -> bool {
        self.manual_hide || self.auto_hide
    }

    pub fn manual_hide(self) -> bool {
        self.manual_hide
    }

    /// Does nothing during an Auto-hide, so nobody sets a Manual hide they can't see.
    pub fn toggle(&mut self) {
        if self.manual_hide {
            self.manual_hide = false;
        } else if !self.auto_hide {
            self.manual_hide = true;
        }
    }

    /// Clears any Manual hide. It never overrides an Auto-hide.
    pub fn show(&mut self) {
        self.manual_hide = false;
    }

    pub fn set_auto_hide(&mut self, applies: bool) {
        self.auto_hide = applies;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auto_hidden() -> Presence {
        let mut presence = Presence::default();
        presence.set_auto_hide(true);
        presence
    }

    #[test]
    fn given_a_normal_launch_when_starting_then_the_pet_is_shown() {
        // Given / When
        let presence = Presence::default();

        // Then
        assert!(!presence.is_hidden());
    }

    #[test]
    fn given_a_relaunch_after_a_manual_hide_when_starting_then_the_pet_is_hidden() {
        // Given / When
        let presence = Presence::with_manual_hide(true);

        // Then
        assert!(presence.is_hidden());
        assert!(presence.manual_hide());
    }

    #[test]
    fn given_a_shown_pet_when_toggling_then_a_manual_hide_is_set() {
        // Given
        let mut presence = Presence::default();

        // When
        presence.toggle();

        // Then
        assert!(presence.is_hidden());
        assert!(presence.manual_hide());
    }

    #[test]
    fn given_a_manual_hide_when_toggling_then_it_is_cleared() {
        // Given
        let mut presence = Presence::with_manual_hide(true);

        // When
        presence.toggle();

        // Then
        assert!(!presence.is_hidden());
        assert!(!presence.manual_hide());
    }

    #[test]
    fn given_only_an_auto_hide_when_toggling_then_nothing_changes() {
        // Given
        let mut presence = auto_hidden();

        // When
        presence.toggle();

        // Then: a Manual hide set now would be one the owner can't see
        assert_eq!(presence, auto_hidden());
    }

    #[test]
    fn given_both_hides_when_toggling_then_the_manual_hide_clears_and_the_pet_stays_hidden() {
        // Given
        let mut presence = auto_hidden();
        presence = Presence {
            manual_hide: true,
            ..presence
        };

        // When
        presence.toggle();

        // Then
        assert!(presence.is_hidden());
        assert!(!presence.manual_hide());
    }

    #[test]
    fn given_a_manual_hide_when_asked_to_show_then_the_pet_is_shown() {
        // Given
        let mut presence = Presence::with_manual_hide(true);

        // When
        presence.show();

        // Then
        assert!(!presence.is_hidden());
    }

    #[test]
    fn given_an_auto_hide_when_asked_to_show_then_the_pet_stays_hidden() {
        // Given
        let mut presence = auto_hidden();

        // When
        presence.show();

        // Then
        assert!(presence.is_hidden());
    }

    #[test]
    fn given_a_manual_hide_when_a_fullscreen_app_opens_and_closes_then_the_pet_stays_hidden() {
        // Given
        let mut presence = Presence::with_manual_hide(true);

        // When
        presence.set_auto_hide(true);
        presence.set_auto_hide(false);

        // Then
        assert!(presence.is_hidden());
        assert!(presence.manual_hide());
    }

    #[test]
    fn given_an_auto_hide_when_the_fullscreen_app_closes_then_the_pet_is_shown() {
        // Given
        let mut presence = auto_hidden();

        // When
        presence.set_auto_hide(false);

        // Then
        assert!(!presence.is_hidden());
    }
}
