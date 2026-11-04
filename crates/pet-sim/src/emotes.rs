//! Emotes: small pieces of art above the Pet that make its motive visible. They belong to the Character, so any Form
//! can show any of the Pack's Emotes. One shows at a time. A one-shot Emote plays once and goes by itself, and a
//! looping one stays until it's hidden.
//!
//! Others ask with an `EmoteRequest`, read once a tick in React and applied in the order they were sent, so the last
//! one wins. The Emote's clock moves on in Act, like the Pet's Animation clock, so it holds while Hidden too. The tick
//! that starts an Emote shows its first Frame with no time played, and time counts from the next tick, in whole ticks
//! as the Pet's does. So each Frame shows from the first tick at or after its start, exactly. Every Frame lasts at
//! least a tick, as the Pack's validator makes sure, so none is ever skipped.

use bevy_ecs::prelude::*;
use pet_core::animation_clock::{self, Playback};
use pet_core::pack::{EmoteDef, EmoteId, EmoteMode, PackDefinition};

use crate::Pet;
use crate::pack::CurrentPack;

/// Asks for an Emote above the Pet to show or go. Requests apply in the order they were sent, so the last one wins.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmoteRequest {
    /// Shows the Emote. It replaces the one showing and starts from its first Frame, except that showing the looping
    /// Emote that is already showing leaves it be.
    Show(EmoteId),
    /// Hides the Emote. It does nothing unless that Emote is the one showing.
    Hide(EmoteId),
}

/// The Emote an Actor is playing, if any, and for how long it has played. It changes every tick while one plays.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EmoteClock(pub Option<PlayingEmote>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayingEmote {
    pub emote: EmoteId,
    /// How many ticks it has played, counting from the tick after the one that showed its first Frame.
    pub played_ticks: u64,
    /// Whether its first Frame has been shown yet.
    pub begun: bool,
}

/// The Emote Frame shown above an Actor, if any. It changes only when the Frame does, or the Emote comes or goes, so
/// readers can react to the change.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShownEmote(pub Option<EmoteFrame>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmoteFrame {
    pub emote: EmoteId,
    pub frame: usize,
}

/// The Emote `emote` names, or `None` with a warning. Ids come from the Pack's names, so an unknown one only comes
/// from a Pack with no Emotes or one that has since been replaced.
fn known(pack: &PackDefinition, emote: EmoteId) -> Option<&EmoteDef> {
    let found = pack.emotes.get(emote.0);
    if found.is_none() {
        log::warn!(
            "Ignored Emote {} that the Pack \"{}\" doesn't have",
            emote.0,
            pack.name
        );
    }
    found
}

/// Applies this tick's requests to the Pet's Emote clock, in the order they were sent. Act then picks the Frame.
pub(crate) fn receive(
    pack: Option<Res<CurrentPack>>,
    mut requests: MessageReader<EmoteRequest>,
    mut pets: Query<&mut EmoteClock, With<Pet>>,
) {
    let requests: Vec<EmoteRequest> = requests.read().copied().collect();
    let Some(pack) = pack else { return };
    for mut clock in &mut pets {
        for request in &requests {
            match *request {
                EmoteRequest::Show(id) => {
                    let Some(emote) = known(&pack.0, id) else {
                        continue;
                    };
                    let showing = clock.0.map(|playing| playing.emote);
                    if emote.mode == EmoteMode::Looping && showing == Some(id) {
                        continue;
                    }
                    clock.0 = Some(PlayingEmote {
                        emote: id,
                        played_ticks: 0,
                        begun: false,
                    });
                }
                EmoteRequest::Hide(id) => {
                    if clock.0.is_some_and(|playing| playing.emote == id) {
                        clock.0 = None;
                    }
                }
            }
        }
    }
}

/// Moves each Emote clock on by a tick and picks its Frame. A one-shot Emote whose last Frame has had its time goes in
/// the same tick, so it's never shown past its end.
pub(crate) fn advance(
    pack: Option<Res<CurrentPack>>,
    mut clocks: Query<(&mut EmoteClock, &mut ShownEmote)>,
) {
    let Some(pack) = pack else { return };
    for (mut clock, mut shown) in &mut clocks {
        let playing = clock.0.and_then(|playing| {
            let emote = pack.0.emotes.get(playing.emote.0)?;
            // The tick that starts it shows its first Frame, as the Pet's first Frame shows before any time passes.
            let played = playing.played_ticks + u64::from(playing.begun);
            let playback = emote.mode.playback();
            let ended = playback == Playback::OneShot
                && animation_clock::has_ended(&emote.durations, played);
            let frame = animation_clock::frame_at(&emote.durations, playback, played);
            let playing = PlayingEmote {
                played_ticks: played,
                begun: true,
                ..playing
            };
            (!ended).then_some((playing, frame))
        });
        if let Some((playing, frame)) = playing {
            clock.0 = Some(playing);
            shown.set_if_neq(ShownEmote(Some(EmoteFrame {
                emote: playing.emote,
                frame,
            })));
        } else {
            clock.set_if_neq(EmoteClock(None));
            shown.set_if_neq(ShownEmote(None));
        }
    }
}
