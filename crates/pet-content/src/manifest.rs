//! The shape of `manifest.toml`, and how a manifest with no errors becomes a `PackDefinition`.
//!
//! Every field is read on its own, all the way down, so each mistake gets its own error and none hides another. A
//! field with a mistake is left out as `None`, and everything beside it is still read and checked. An entry, such as an
//! Action, always keeps its name, so nothing that refers to it reports a second error.

use std::collections::BTreeMap;
use std::time::Duration;

use pet_core::pack::{
    ActionDef, ActionId, Anchor, AnimationDef, AnimationId, Body, CellSize, EmoteDef, EmoteMode,
    FormDef, FormId, Marker, PackDefinition, Shadow, Situation,
};
use serde::de::DeserializeOwned;
use toml::Value;

use crate::PackError;

pub const MANIFEST: &str = "manifest.toml";

/// The one `format_version` this build understands.
pub const FORMAT_VERSION: i64 = 1;

/// A pixel catches clicks from this alpha up, unless the Pack says otherwise.
pub const DEFAULT_HIT_MASK_THRESHOLD: u32 = 128;

/// What the Definition is made from. Only a manifest with no errors at all becomes one.
const ONLY_WITHOUT_ERRORS: &str = "the Definition is only made from a manifest with no errors";

/// Named entries, such as a Form's Actions. An entry is `None` when it isn't a table at all.
pub type Entries<T> = BTreeMap<String, Option<T>>;

/// A manifest as read. Wherever a value is `None`, its mistake has already been reported. A table of entries that's
/// missing is empty, but one of the wrong kind is `None`, so nothing is judged against it.
#[derive(Debug)]
pub struct Manifest {
    /// Empty when it's missing.
    pub name: String,
    pub starting_form: Option<String>,
    /// The default when the field isn't there.
    pub hit_mask_threshold: Option<u32>,
    pub forms: Option<Entries<Form>>,
    pub emotes: Option<Entries<Emote>>,
}

#[derive(Debug)]
pub struct Form {
    pub cell: Option<Size>,
    pub anchor: Option<Point>,
    pub body: Option<BodyBox>,
    pub default_action: Option<String>,
    /// `None` when there isn't one, or it has a mistake.
    pub shadow: Option<ShadowShape>,
    pub actions: Option<Entries<Action>>,
    /// Which Animation each Situation plays.
    pub situations: Option<Entries<String>>,
    pub animations: Option<Entries<Animation>>,
}

#[derive(Clone, Copy, Debug)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct BodyBox {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct ShadowShape {
    pub width: u32,
    pub colour: String,
}

#[derive(Debug)]
pub struct Action {
    pub animation: Option<String>,
    /// `None` when there isn't one, or it has a mistake.
    pub body: Option<BodyBox>,
}

#[derive(Debug)]
pub struct Animation {
    pub sheet: Option<String>,
    pub durations_ms: Option<Vec<u32>>,
    /// The markers that read cleanly.
    pub markers: Vec<MarkerShape>,
}

#[derive(Debug)]
pub struct MarkerShape {
    pub frame: u32,
    pub name: String,
}

#[derive(Debug)]
pub struct Emote {
    pub sheet: Option<String>,
    pub cell: Option<Size>,
    pub durations_ms: Option<Vec<u32>>,
    pub mode: Option<String>,
}

/// The kind of a TOML value with "a" or "an" in front, such as "an integer".
fn kind(value: &Value) -> String {
    let name = value.type_str();
    let article = if name.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {name}")
}

/// Reads one table a field at a time, adding an error for each mistake.
struct Fields<'a> {
    table: &'a toml::Table,
    path: String,
    errors: &'a mut Vec<PackError>,
}

/// Reads one value at `path`, returning `None` when it can't be used.
type Reader<T> = fn(&Value, String, &mut Vec<PackError>) -> Option<T>;

impl<'a> Fields<'a> {
    /// Starts on `value`, which must be a table. Each field in it that isn't one of the `known` ones is an error, and
    /// the known ones are still read.
    fn read(
        value: &'a Value,
        path: String,
        known: &[&str],
        errors: &'a mut Vec<PackError>,
    ) -> Option<Self> {
        let Some(table) = value.as_table() else {
            let message = format!("should be a table, but is {}", kind(value));
            errors.push(PackError::new(MANIFEST, path, message));
            return None;
        };
        let mut fields = Self {
            table,
            path,
            errors,
        };
        for key in table.keys().filter(|key| !known.contains(&key.as_str())) {
            let message = format!("isn't a field here. The fields are {}", known.join(", "));
            fields.error(key, message);
        }
        Some(fields)
    }

    fn path_of(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_owned()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    fn error(&mut self, key: &str, message: impl Into<String>) {
        let path = self.path_of(key);
        self.errors.push(PackError::new(MANIFEST, path, message));
    }

    /// A plain value, such as a number, some text or a list of numbers.
    fn required<T: DeserializeOwned>(&mut self, key: &str) -> Option<T> {
        let Some(value) = self.table.get(key) else {
            self.error(key, "is missing");
            return None;
        };
        match value.clone().try_into() {
            Ok(read) => Some(read),
            Err(error) => {
                self.error(key, error.message());
                None
            }
        }
    }

    /// `Some(None)` when the field isn't there, and `None` when it has a mistake.
    fn optional<T: DeserializeOwned>(&mut self, key: &str) -> Option<Option<T>> {
        if self.table.contains_key(key) {
            self.required(key).map(Some)
        } else {
            Some(None)
        }
    }

    fn required_with<T>(&mut self, key: &str, read: Reader<T>) -> Option<T> {
        let Some(value) = self.table.get(key) else {
            self.error(key, "is missing");
            return None;
        };
        read(value, self.path_of(key), self.errors)
    }

    /// `None` when the field isn't there, or has a mistake.
    fn optional_with<T>(&mut self, key: &str, read: Reader<T>) -> Option<T> {
        let value = self.table.get(key)?;
        read(value, self.path_of(key), self.errors)
    }

    /// A list of tables, keeping those that read cleanly. Empty when the field isn't there.
    fn list_with<T>(&mut self, key: &str, read: Reader<T>) -> Vec<T> {
        let path = self.path_of(key);
        match self.table.get(key) {
            None => Vec::new(),
            Some(Value::Array(items)) => items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| read(item, format!("{path}[{index}]"), self.errors))
                .collect(),
            Some(other) => {
                self.error(key, format!("should be a list, but is {}", kind(other)));
                Vec::new()
            }
        }
    }

    /// A table of named entries, each read on its own. Empty when the field isn't there, and `None` when it isn't a
    /// table, so nothing is judged against a list that was never read.
    fn entries<T>(&mut self, key: &str, read: Reader<T>) -> Option<Entries<T>> {
        let path = self.path_of(key);
        match self.table.get(key) {
            None => Some(BTreeMap::new()),
            Some(Value::Table(entries)) => Some(
                entries
                    .iter()
                    .map(|(name, entry)| {
                        let path = format!("{path}.{name}");
                        (name.clone(), read(entry, path, self.errors))
                    })
                    .collect(),
            ),
            Some(other) => {
                self.error(key, format!("should be a table, but is {}", kind(other)));
                None
            }
        }
    }
}

fn read_size(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<Size> {
    let mut fields = Fields::read(value, path, &["width", "height"], errors)?;
    let width = fields.required("width");
    let height = fields.required("height");
    Some(Size {
        width: width?,
        height: height?,
    })
}

fn read_point(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<Point> {
    let mut fields = Fields::read(value, path, &["x", "y"], errors)?;
    let x = fields.required("x");
    let y = fields.required("y");
    Some(Point { x: x?, y: y? })
}

fn read_body(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<BodyBox> {
    let mut fields = Fields::read(value, path, &["x", "y", "width", "height"], errors)?;
    let x = fields.required("x");
    let y = fields.required("y");
    let width = fields.required("width");
    let height = fields.required("height");
    Some(BodyBox {
        x: x?,
        y: y?,
        width: width?,
        height: height?,
    })
}

fn read_shadow(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<ShadowShape> {
    let mut fields = Fields::read(value, path, &["width", "colour"], errors)?;
    let width = fields.required("width");
    let colour = fields.required("colour");
    Some(ShadowShape {
        width: width?,
        colour: colour?,
    })
}

fn read_action(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<Action> {
    let mut fields = Fields::read(value, path, &["animation", "body"], errors)?;
    Some(Action {
        animation: fields.required("animation"),
        body: fields.optional_with("body", read_body),
    })
}

fn read_situation(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<String> {
    match value.as_str() {
        Some(animation) => Some(animation.to_owned()),
        None => {
            let message = format!("should name an Animation, but is {}", kind(value));
            errors.push(PackError::new(MANIFEST, path, message));
            None
        }
    }
}

fn read_marker(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<MarkerShape> {
    let mut fields = Fields::read(value, path, &["frame", "name"], errors)?;
    let frame = fields.required("frame");
    let name = fields.required("name");
    Some(MarkerShape {
        frame: frame?,
        name: name?,
    })
}

fn read_animation(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<Animation> {
    let known = ["sheet", "durations_ms", "markers"];
    let mut fields = Fields::read(value, path, &known, errors)?;
    Some(Animation {
        sheet: fields.required("sheet"),
        durations_ms: fields.required("durations_ms"),
        markers: fields.list_with("markers", read_marker),
    })
}

fn read_emote(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<Emote> {
    let known = ["sheet", "cell", "durations_ms", "mode"];
    let mut fields = Fields::read(value, path, &known, errors)?;
    Some(Emote {
        sheet: fields.required("sheet"),
        cell: fields.required_with("cell", read_size),
        durations_ms: fields.required("durations_ms"),
        mode: fields.required("mode"),
    })
}

fn read_form(value: &Value, path: String, errors: &mut Vec<PackError>) -> Option<Form> {
    let known = [
        "cell",
        "anchor",
        "body",
        "default_action",
        "shadow",
        "actions",
        "situations",
        "animations",
    ];
    let mut fields = Fields::read(value, path, &known, errors)?;
    Some(Form {
        cell: fields.required_with("cell", read_size),
        anchor: fields.required_with("anchor", read_point),
        body: fields.required_with("body", read_body),
        default_action: fields.required("default_action"),
        shadow: fields.optional_with("shadow", read_shadow),
        actions: fields.entries("actions", read_action),
        situations: fields.entries("situations", read_situation),
        animations: fields.entries("animations", read_animation),
    })
}

/// Reads the manifest's text. It gives up with one error only when the text isn't TOML or names a format this build
/// doesn't know. Otherwise it returns what it could read, with an error for every mistake in the shape.
pub fn parse(text: &str) -> Result<(Manifest, Vec<PackError>), Vec<PackError>> {
    let table: toml::Table = text.parse().map_err(|error: toml::de::Error| {
        let message = format!("isn't valid TOML: {error}");
        vec![PackError::new(MANIFEST, "", message)]
    })?;
    // The version decides which fields exist and what they mean, so in a format this build doesn't know, nothing
    // else in the file can be judged.
    check_format_version(table.get("format_version")).map_err(|error| vec![error])?;

    let mut errors = Vec::new();
    let root = Value::Table(table);
    let known = [
        "format_version",
        "name",
        "starting_form",
        "hit_mask_threshold",
        "forms",
        "emotes",
    ];
    let mut fields = Fields::read(&root, String::new(), &known, &mut errors)
        .expect("a TOML file is always a table");
    let name: Option<String> = fields.required("name");
    let manifest = Manifest {
        name: name.unwrap_or_default(),
        starting_form: fields.required("starting_form"),
        hit_mask_threshold: fields
            .optional("hit_mask_threshold")
            .map(|threshold| threshold.unwrap_or(DEFAULT_HIT_MASK_THRESHOLD)),
        forms: fields.entries("forms", read_form),
        emotes: fields.entries("emotes", read_emote),
    };
    Ok((manifest, errors))
}

fn check_format_version(version: Option<&Value>) -> Result<(), PackError> {
    let error = |message: String| PackError::new(MANIFEST, "format_version", message);
    match version {
        Some(Value::Integer(FORMAT_VERSION)) => Ok(()),
        Some(other) => Err(error(format!(
            "{other} isn't a format this build understands: it reads {FORMAT_VERSION}"
        ))),
        None => Err(error(format!(
            "is missing: add `format_version = {FORMAT_VERSION}`"
        ))),
    }
}

/// Reads `#rrggbb` or `#rrggbbaa` as RGBA8. Without an alpha it's fully opaque.
pub fn parse_colour(text: &str) -> Option<[u8; 4]> {
    let hex = text.strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.is_ascii() {
        return None;
    }
    let byte = |at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
    let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
    Some([byte(0)?, byte(2)?, byte(4)?, alpha])
}

/// An entry of a manifest with no errors, which always has its contents.
pub fn complete<T>(entry: &Option<T>) -> &T {
    entry.as_ref().expect(ONLY_WITHOUT_ERRORS)
}

/// The place of `name` in `names`, which is the number it becomes.
fn index_of<T>(names: &BTreeMap<String, T>, name: &str) -> usize {
    names
        .keys()
        .position(|key| key == name)
        .expect(ONLY_WITHOUT_ERRORS)
}

fn durations(durations_ms: &[u32]) -> Vec<Duration> {
    durations_ms
        .iter()
        .map(|&ms| Duration::from_millis(ms.into()))
        .collect()
}

fn cell(size: Size) -> CellSize {
    CellSize {
        width: size.width,
        height: size.height,
    }
}

fn body(body: BodyBox) -> Body {
    Body {
        x: body.x,
        y: body.y,
        width: body.width,
        height: body.height,
    }
}

/// Turns a manifest with no errors, from here or from `validate::check`, into the Definition. Names become numbers in
/// name order, the same order `images` cuts the Frames in.
pub fn to_definition(manifest: &Manifest) -> PackDefinition {
    let all_forms = complete(&manifest.forms);
    let forms = all_forms
        .iter()
        .map(|(name, form)| form_definition(name, complete(form)))
        .collect();
    let emotes = complete(&manifest.emotes)
        .iter()
        .map(|(name, emote)| {
            let emote = complete(emote);
            EmoteDef {
                name: name.clone(),
                cell: cell(*complete(&emote.cell)),
                durations: durations(complete(&emote.durations_ms)),
                mode: EmoteMode::from_name(complete(&emote.mode)).expect(ONLY_WITHOUT_ERRORS),
            }
        })
        .collect();
    let starting_form = complete(&manifest.starting_form);
    let threshold = *complete(&manifest.hit_mask_threshold);
    PackDefinition {
        name: manifest.name.clone(),
        starting_form: FormId(index_of(all_forms, starting_form)),
        forms,
        emotes,
        hit_mask_threshold: threshold.try_into().expect(ONLY_WITHOUT_ERRORS),
    }
}

fn form_definition(name: &str, form: &Form) -> FormDef {
    let actions = complete(&form.actions);
    let animations = complete(&form.animations);
    let animation_id = |name: &str| AnimationId(index_of(animations, name));
    let anchor = complete(&form.anchor);
    FormDef {
        name: name.to_owned(),
        cell: cell(*complete(&form.cell)),
        anchor: Anchor {
            x: anchor.x,
            y: anchor.y,
        },
        body: body(*complete(&form.body)),
        default_action: ActionId(index_of(actions, complete(&form.default_action))),
        shadow: form.shadow.as_ref().map(|shadow| Shadow {
            width: shadow.width,
            colour: parse_colour(&shadow.colour).expect(ONLY_WITHOUT_ERRORS),
        }),
        actions: actions
            .iter()
            .map(|(name, action)| {
                let action = complete(action);
                ActionDef {
                    name: name.clone(),
                    animation: animation_id(complete(&action.animation)),
                    body: action.body.map(body),
                }
            })
            .collect(),
        animations: animations
            .iter()
            .map(|(name, animation)| {
                let animation = complete(animation);
                AnimationDef {
                    name: name.clone(),
                    durations: durations(complete(&animation.durations_ms)),
                    markers: animation
                        .markers
                        .iter()
                        .map(|marker| Marker {
                            frame: marker.frame as usize,
                            name: marker.name.clone(),
                        })
                        .collect(),
                }
            })
            .collect(),
        situations: complete(&form.situations)
            .iter()
            .map(|(name, animation)| {
                let situation = Situation::from_name(name).expect(ONLY_WITHOUT_ERRORS);
                (situation, animation_id(complete(animation)))
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
        format_version = 1
        name = "Test"
        starting_form = "blob"

        [forms.blob]
        cell = { width = 2, height = 2 }
        anchor = { x = 1, y = 2 }
        body = { x = -1, y = -2, width = 2, height = 2 }
        default_action = "idle"

        [forms.blob.actions.idle]
        animation = "idle"

        [forms.blob.animations.idle]
        sheet = "idle.png"
        durations_ms = [100]
    "#;

    fn fields(errors: &[PackError]) -> Vec<&str> {
        errors.iter().map(|error| error.field.as_str()).collect()
    }

    /// The shape errors in `text`, which must be TOML in a known format.
    fn shape_errors(text: &str) -> Vec<PackError> {
        parse(text).expect("TOML in a known format").1
    }

    #[test]
    fn given_a_well_formed_manifest_when_parsing_then_every_entry_is_read_and_nothing_is_wrong() {
        // Given
        let text = MINIMAL;

        // When
        let (manifest, errors) = parse(text).expect("TOML in a known format");

        // Then
        assert_eq!(errors, []);
        assert_eq!(manifest.name, "Test");
        assert_eq!(
            manifest.hit_mask_threshold,
            Some(DEFAULT_HIT_MASK_THRESHOLD)
        );
        let form = complete(&complete(&manifest.forms)["blob"]);
        let action = complete(&complete(&form.actions)["idle"]);
        assert_eq!(action.animation.as_deref(), Some("idle"));
        let animation = complete(&complete(&form.animations)["idle"]);
        assert_eq!(animation.durations_ms.as_deref(), Some([100].as_slice()));
    }

    #[test]
    fn given_text_that_is_not_toml_when_parsing_then_one_error_says_so_and_names_the_line() {
        // Given
        let text = "format_version = 1\nname = \"unfinished";

        // When
        let errors = parse(text).expect_err("not TOML");

        // Then
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].file, MANIFEST);
        assert!(errors[0].message.contains("isn't valid TOML"), "{errors:?}");
        assert!(errors[0].message.contains("line 2"), "{errors:?}");
    }

    #[test]
    fn given_a_format_version_this_build_does_not_know_when_parsing_then_only_that_is_reported() {
        // Given: the rest would be wrong too, but can't be judged in an unknown format
        let text = "format_version = 2\nsurprise = true";

        // When
        let errors = parse(text).expect_err("unknown format");

        // Then
        assert_eq!(fields(&errors), ["format_version"]);
    }

    #[test]
    fn given_no_format_version_when_parsing_then_it_is_reported_as_missing() {
        // Given
        let text = MINIMAL.replace("format_version = 1", "");

        // When
        let errors = parse(&text).expect_err("no format version");

        // Then
        assert_eq!(fields(&errors), ["format_version"]);
        assert!(errors[0].message.contains("missing"), "{errors:?}");
    }

    #[test]
    fn given_unknown_fields_in_two_entries_when_parsing_then_both_are_reported_with_where_they_are()
    {
        // Given: an extra field in the Action and another in the Animation
        let text = MINIMAL
            .replace("animation = \"idle\"", "animation = \"idle\"\nspeed = 2")
            .replace("durations_ms = [100]", "durations_ms = [100]\nloop = true");

        // When
        let errors = shape_errors(&text);

        // Then
        assert_eq!(
            fields(&errors),
            [
                "forms.blob.actions.idle.speed",
                "forms.blob.animations.idle.loop"
            ]
        );
    }

    #[test]
    fn given_two_mistakes_in_one_small_table_when_parsing_then_both_are_reported() {
        // Given: the cell's height is text, and it has a field it shouldn't
        let text = MINIMAL.replace(
            "cell = { width = 2, height = 2 }",
            "cell = { width = 2, height = \"2\", depth = 1 }",
        );

        // When
        let errors = shape_errors(&text);

        // Then
        assert_eq!(
            fields(&errors),
            ["forms.blob.cell.depth", "forms.blob.cell.height"]
        );
    }

    #[test]
    fn given_an_unknown_field_and_a_missing_one_at_the_top_when_parsing_then_both_are_reported() {
        // Given
        let text = MINIMAL
            .replace("name = \"Test\"", "author = \"me\"")
            .replace("starting_form = \"blob\"", "");

        // When
        let errors = shape_errors(&text);

        // Then
        assert_eq!(fields(&errors), ["author", "name", "starting_form"]);
        assert!(errors[0].message.contains("isn't a field"), "{errors:?}");
        assert_eq!(errors[1].message, "is missing");
    }

    #[test]
    fn given_a_mistake_in_an_action_when_parsing_then_the_action_keeps_its_name_and_its_other_fields()
     {
        // Given: the Animation is a number, and the Body is fine
        let text = MINIMAL.replace(
            "animation = \"idle\"",
            "animation = 3
body = { x = 0, y = 0, width = 1, height = 1 }",
        );

        // When
        let (manifest, errors) = parse(&text).expect("TOML in a known format");

        // Then
        assert_eq!(fields(&errors), ["forms.blob.actions.idle.animation"]);
        assert_eq!(
            errors[0].message,
            "invalid type: integer `3`, expected a string"
        );
        let form = complete(&complete(&manifest.forms)["blob"]);
        let action = complete(&complete(&form.actions)["idle"]);
        assert!(action.animation.is_none());
        assert!(action.body.is_some());
        assert_eq!(form.default_action.as_deref(), Some("idle"));
    }

    #[test]
    fn given_two_actions_with_the_same_name_when_parsing_then_the_error_names_the_line_of_the_second()
     {
        // Given: names are unique within their list, and TOML itself refuses a repeated table
        let text = format!("{MINIMAL}\n[forms.blob.actions.idle]\nanimation = \"idle\"\n");

        // When
        let errors = parse(&text).expect_err("a repeated name");

        // Then
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("duplicate"), "{errors:?}");
        assert!(errors[0].message.contains("line 19"), "{errors:?}");
    }

    #[test]
    fn given_colours_with_and_without_alpha_when_reading_them_then_both_become_rgba() {
        // Given
        let colours = ["#22203480", "#ff0000", "22203480", "#12345", "#gg0000"];

        // When
        let read = colours.map(parse_colour);

        // Then
        assert_eq!(
            read,
            [
                Some([0x22, 0x20, 0x34, 0x80]),
                Some([255, 0, 0, 255]),
                None,
                None,
                None
            ]
        );
    }

    #[test]
    fn given_a_manifest_with_no_errors_when_making_the_definition_then_names_become_numbers_in_name_order()
     {
        // Given: "walk" sorts after "idle", and the default Action is "walk"
        let walk = "\n[forms.blob.actions.walk]\nanimation = \"walk\"\n\
                    [forms.blob.animations.walk]\nsheet = \"walk.png\"\ndurations_ms = [80, 90]\n";
        let text = MINIMAL.replace("default_action = \"idle\"", "default_action = \"walk\"") + walk;
        let (manifest, errors) = parse(&text).expect("TOML in a known format");
        assert_eq!(errors, []);

        // When
        let definition = to_definition(&manifest);

        // Then
        let form = definition.form(definition.starting_form);
        assert_eq!(form.name, "blob");
        assert_eq!(form.default_action, ActionId(1));
        assert_eq!(form.action(ActionId(1)).name, "walk");
        let walk = form.animation(form.default_animation());
        assert_eq!(walk.name, "walk");
        assert_eq!(
            walk.durations,
            [Duration::from_millis(80), Duration::from_millis(90)]
        );
        assert_eq!(definition.hit_mask_threshold, 128);
    }
}
