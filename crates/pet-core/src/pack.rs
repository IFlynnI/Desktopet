//! The Definition of a Pack: everything one Character is made of, as plain read-only data. `pet-content` reads it from
//! a Pack folder and checks it, so everything here can assume it's valid. Nothing here reads files.

use std::time::Duration;

/// A Form, by its place in `PackDefinition::forms`. Names from the Pack become numbers like this one, so the rest of
/// the program never compares strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormId(pub usize);

/// An Action, by its place in its Form's `actions`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActionId(pub usize);

/// An Animation, by its place in its Form's `animations`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AnimationId(pub usize);

/// An Emote, by its place in `PackDefinition::emotes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EmoteId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct PackDefinition {
    pub name: String,
    pub starting_form: FormId,
    pub forms: Vec<FormDef>,
    /// Shared by every Form. A Pack may have none.
    pub emotes: Vec<EmoteDef>,
    /// A pixel catches clicks from this alpha up.
    pub hit_mask_threshold: u8,
}

impl PackDefinition {
    pub fn form(&self, id: FormId) -> &FormDef {
        &self.forms[id.0]
    }

    pub fn emote(&self, id: EmoteId) -> &EmoteDef {
        &self.emotes[id.0]
    }
}

/// The size of every Frame of a Form or an Emote, in art pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellSize {
    pub width: u32,
    pub height: u32,
}

/// Where the feet meet the ground, in art pixels from the cell's top-left, measured between pixels rather than naming
/// one. So `x` may equal the cell's width, the right edge, and `y` its height, the bottom edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub x: u32,
    pub y: u32,
}

/// The rectangle the Pet takes up in the world, in art pixels relative to the Anchor. Never used for clicks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// The drop shadow under the Anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shadow {
    /// In art pixels.
    pub width: u32,
    /// RGBA8.
    pub colour: [u8; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormDef {
    pub name: String,
    pub cell: CellSize,
    pub anchor: Anchor,
    pub body: Body,
    pub default_action: ActionId,
    pub shadow: Option<Shadow>,
    pub actions: Vec<ActionDef>,
    pub animations: Vec<AnimationDef>,
    /// The Animation each mapped Situation plays. A Situation that isn't here plays the default Action.
    pub situations: Vec<(Situation, AnimationId)>,
}

impl FormDef {
    pub fn action(&self, id: ActionId) -> &ActionDef {
        &self.actions[id.0]
    }

    pub fn animation(&self, id: AnimationId) -> &AnimationDef {
        &self.animations[id.0]
    }

    /// The Animation this Form plays when nothing else is going on.
    pub fn default_animation(&self) -> AnimationId {
        self.action(self.default_action).animation
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionDef {
    pub name: String,
    pub animation: AnimationId,
    /// A Body of its own while this Action runs, such as a smaller one when curled up asleep.
    pub body: Option<Body>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationDef {
    pub name: String,
    /// One per Frame, in order.
    pub durations: Vec<Duration>,
    pub markers: Vec<Marker>,
}

/// A named moment in an Animation, such as a footstep, for sound to follow later.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marker {
    pub frame: usize,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmoteDef {
    pub name: String,
    pub cell: CellSize,
    /// One per Frame, in order.
    pub durations: Vec<Duration>,
    pub mode: EmoteMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmoteMode {
    /// Plays once, then goes.
    OneShot,
    /// Stays until the motive passes.
    Looping,
}

impl EmoteMode {
    /// The mode a Pack names, or `None` for a name that isn't one.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "one-shot" => Some(Self::OneShot),
            "looping" => Some(Self::Looping),
            _ => None,
        }
    }
}

/// Something that happens to the Pet rather than something it chooses. This is the one fixed list in a Pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Situation {
    Held,
    Falling,
    Landing,
}

impl Situation {
    pub const ALL: [Situation; 3] = [Situation::Held, Situation::Falling, Situation::Landing];

    /// The name a Pack uses for this Situation.
    pub fn name(self) -> &'static str {
        match self {
            Self::Held => "held",
            Self::Falling => "falling",
            Self::Landing => "landing",
        }
    }

    /// The Situation a Pack names, or `None` for a name that isn't one.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|situation| situation.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn animation(name: &str) -> AnimationDef {
        AnimationDef {
            name: name.to_owned(),
            durations: vec![Duration::from_millis(250)],
            markers: Vec::new(),
        }
    }

    fn action(name: &str, animation: usize) -> ActionDef {
        ActionDef {
            name: name.to_owned(),
            animation: AnimationId(animation),
            body: None,
        }
    }

    #[test]
    fn given_a_form_whose_default_action_is_second_when_asking_for_its_default_animation_then_it_is_that_actions_animation()
     {
        // Given: "sleep" is the default Action, and it plays the "sleep" Animation
        let form = FormDef {
            name: "hatchling".to_owned(),
            cell: CellSize {
                width: 24,
                height: 18,
            },
            anchor: Anchor { x: 12, y: 18 },
            body: Body {
                x: -9,
                y: -15,
                width: 18,
                height: 15,
            },
            default_action: ActionId(1),
            shadow: None,
            actions: vec![action("idle", 0), action("sleep", 1)],
            animations: vec![animation("idle"), animation("sleep")],
            situations: Vec::new(),
        };

        // When
        let default = form.default_animation();

        // Then
        assert_eq!(form.animation(default).name, "sleep");
    }

    #[test]
    fn given_each_situation_name_when_reading_it_then_it_is_that_situation_and_nothing_else_is_one()
    {
        // Given
        let names = ["held", "falling", "landing", "walk", "Held"];

        // When
        let situations = names.map(Situation::from_name);

        // Then
        assert_eq!(
            situations,
            [
                Some(Situation::Held),
                Some(Situation::Falling),
                Some(Situation::Landing),
                None,
                None
            ]
        );
    }

    #[test]
    fn given_each_emote_mode_name_when_reading_it_then_it_is_that_mode_and_nothing_else_is_one() {
        // Given
        let names = ["one-shot", "looping", "once"];

        // When
        let modes = names.map(EmoteMode::from_name);

        // Then
        assert_eq!(
            modes,
            [Some(EmoteMode::OneShot), Some(EmoteMode::Looping), None]
        );
    }
}
