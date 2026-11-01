//! The tray icon and its menu. `tray-icon`'s types can't leave the thread that made them, which suits this: they all
//! live on the shell thread, and their callbacks run there too, inside its message loop or the menu's.

use std::cell::RefCell;
use std::env;
use std::path::PathBuf;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use super::autostart::{Autostart, RUN_KEY, settled_state};
use crate::{ShellEvent, TrayIconImage};

const SHOW_HIDE: &str = "show-hide";
const LAUNCH_ON_STARTUP: &str = "launch-on-startup";
const QUIT: &str = "quit";

struct LaunchOnStartup {
    item: CheckMenuItem,
    exe: PathBuf,
}

thread_local! {
    /// A menu handler has to be safe to move between threads, and a menu item isn't, so the handler finds it here.
    static LAUNCH_ON_STARTUP_ITEM: RefCell<Option<LaunchOnStartup>> = const { RefCell::new(None) };
}

/// The shell thread writes this itself, so the engine never hears about it. The menu has already flipped the tick by
/// the time this runs.
fn launch_on_startup_clicked() {
    LAUNCH_ON_STARTUP_ITEM.with_borrow(|launch| {
        if let Some(LaunchOnStartup { item, exe }) = launch {
            let autostart = Autostart::at(RUN_KEY);
            let shown = settled_state(item.is_checked(), |on| autostart.set(on, exe));
            item.set_checked(shown);
        }
    });
}

fn tooltip(hotkey_available: bool) -> &'static str {
    if hotkey_available {
        "Desktop Pet"
    } else {
        "Desktop Pet (Ctrl+Alt+P unavailable)"
    }
}

/// A click counts when the button comes back up, the way a button on screen does.
fn event_for_click(event: &TrayIconEvent) -> Option<ShellEvent> {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => Some(ShellEvent::Toggle),
        _ => None,
    }
}

fn event_for_menu_item(id: &MenuId) -> Option<ShellEvent> {
    match id.as_ref() {
        SHOW_HIDE => Some(ShellEvent::Toggle),
        QUIT => Some(ShellEvent::Quit),
        _ => None,
    }
}

fn build(image: TrayIconImage, hotkey_available: bool) -> Result<TrayIcon, String> {
    let exe = env::current_exe().map_err(|error| error.to_string())?;
    let ticked = Autostart::at(RUN_KEY).is_on(&exe);
    // Made in menu order, because the item numbers follow it and the tests send those numbers.
    let show_hide = MenuItem::with_id(SHOW_HIDE, "Show/Hide", true, None);
    let launch_on_startup =
        CheckMenuItem::with_id(LAUNCH_ON_STARTUP, "Launch on startup", true, ticked, None);
    let quit = MenuItem::with_id(QUIT, "Quit", true, None);
    let menu = Menu::new();
    menu.append_items(&[&show_hide, &launch_on_startup, &quit])
        .map_err(|error| error.to_string())?;
    LAUNCH_ON_STARTUP_ITEM.set(Some(LaunchOnStartup {
        item: launch_on_startup,
        exe,
    }));
    let icon = Icon::from_rgba(image.rgba, image.width, image.height)
        .map_err(|error| error.to_string())?;
    TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip(tooltip(hotkey_available))
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()
        .map_err(|error| error.to_string())
}

/// The icon, which stays on the taskbar until this is dropped.
pub struct Tray {
    _icon: TrayIcon,
}

impl Drop for Tray {
    fn drop(&mut self) {
        LAUNCH_ON_STARTUP_ITEM.take();
    }
}

/// Adds the icon. Without one the hotkey still works, so a failure is logged and the app carries on.
pub fn add(image: TrayIconImage, hotkey_available: bool, notify: fn(ShellEvent)) -> Option<Tray> {
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let Some(event) = event_for_click(&event) {
            notify(event);
        }
    }));
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id() == LAUNCH_ON_STARTUP {
            launch_on_startup_clicked();
        } else if let Some(event) = event_for_menu_item(event.id()) {
            notify(event);
        }
    }));
    build(image, hotkey_available)
        .map(|icon| Tray { _icon: icon })
        .inspect_err(|error| log::error!("couldn't add the tray icon: {error}"))
        .ok()
}

#[cfg(test)]
mod tests {
    use tray_icon::dpi::PhysicalPosition;
    use tray_icon::{Rect, TrayIconId};

    use super::*;

    fn click(button: MouseButton, button_state: MouseButtonState) -> TrayIconEvent {
        TrayIconEvent::Click {
            id: TrayIconId::new("test"),
            position: PhysicalPosition::default(),
            rect: Rect::default(),
            button,
            button_state,
        }
    }

    #[test]
    fn given_the_left_button_comes_up_when_reading_the_click_then_it_toggles() {
        // Given
        let event = click(MouseButton::Left, MouseButtonState::Up);

        // When
        let action = event_for_click(&event);

        // Then
        assert_eq!(action, Some(ShellEvent::Toggle));
    }

    #[test]
    fn given_the_left_button_goes_down_when_reading_the_click_then_nothing_happens_yet() {
        // Given: a toggle here too would undo itself on the way back up
        let event = click(MouseButton::Left, MouseButtonState::Down);

        // When
        let action = event_for_click(&event);

        // Then
        assert_eq!(action, None);
    }

    #[test]
    fn given_a_right_click_when_reading_the_click_then_the_engine_hears_nothing() {
        // Given: tray-icon opens the menu for this itself
        let event = click(MouseButton::Right, MouseButtonState::Up);

        // When
        let action = event_for_click(&event);

        // Then
        assert_eq!(action, None);
    }

    #[test]
    fn given_the_show_hide_item_when_reading_the_menu_then_it_toggles() {
        // Given
        let id = MenuId::new(SHOW_HIDE);

        // When
        let action = event_for_menu_item(&id);

        // Then: the label stays "Show/Hide" whatever the Pet is doing, so this follows the same rule as a click
        assert_eq!(action, Some(ShellEvent::Toggle));
    }

    #[test]
    fn given_the_quit_item_when_reading_the_menu_then_it_quits() {
        // Given
        let id = MenuId::new(QUIT);

        // When
        let action = event_for_menu_item(&id);

        // Then
        assert_eq!(action, Some(ShellEvent::Quit));
    }

    #[test]
    fn given_the_hotkey_is_available_when_writing_the_tooltip_then_it_is_just_the_name() {
        // Given / When
        let text = tooltip(true);

        // Then
        assert_eq!(text, "Desktop Pet");
    }

    #[test]
    fn given_the_hotkey_is_taken_when_writing_the_tooltip_then_it_says_so() {
        // Given / When
        let text = tooltip(false);

        // Then
        assert_eq!(text, "Desktop Pet (Ctrl+Alt+P unavailable)");
    }
}
