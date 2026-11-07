//! Reading a TOML table one field at a time, so each mistake gets its own error and none hides another. The manifest
//! and the config files are all read this way.
//!
//! A mistake here doesn't know which file it's in yet: whoever reads the file adds that, so the same reader, such as
//! the one for a Material, works in any of them.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use toml::Value;

/// One mistake, and where in its file.
#[derive(Debug)]
pub struct Mistake {
    /// Such as `forms.hatchling.default_action`. Empty when it's about the whole file.
    pub field: String,
    pub message: String,
}

impl Mistake {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Named entries, such as a Form's Actions. An entry is `None` when it isn't a table at all.
pub type Entries<T> = BTreeMap<String, Option<T>>;

/// The kind of a TOML value with "a" or "an" in front, such as "an integer".
pub fn kind(value: &Value) -> String {
    let name = value.type_str();
    let article = if name.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {name}")
}

/// Reads one table a field at a time, adding a mistake for each one.
pub struct Fields<'a> {
    table: &'a toml::Table,
    path: String,
    errors: &'a mut Vec<Mistake>,
}

/// Reads one value at `path`, returning `None` when it can't be used.
pub type Reader<T> = fn(&Value, String, &mut Vec<Mistake>) -> Option<T>;

impl<'a> Fields<'a> {
    /// Starts on `value`, which must be a table. Each field in it that isn't one of the `known` ones is a mistake, and
    /// the known ones are still read.
    pub fn read(
        value: &'a Value,
        path: String,
        known: &[&str],
        errors: &'a mut Vec<Mistake>,
    ) -> Option<Self> {
        let Some(table) = value.as_table() else {
            let message = format!("should be a table, but is {}", kind(value));
            errors.push(Mistake::new(path, message));
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

    pub fn path_of(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_owned()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    pub fn error(&mut self, key: &str, message: impl Into<String>) {
        let path = self.path_of(key);
        self.errors.push(Mistake::new(path, message));
    }

    /// A plain value, such as a number, some text or a list of numbers.
    pub fn required<T: DeserializeOwned>(&mut self, key: &str) -> Option<T> {
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
    pub fn optional<T: DeserializeOwned>(&mut self, key: &str) -> Option<Option<T>> {
        if self.table.contains_key(key) {
            self.required(key).map(Some)
        } else {
            Some(None)
        }
    }

    pub fn required_with<T>(&mut self, key: &str, read: Reader<T>) -> Option<T> {
        let Some(value) = self.table.get(key) else {
            self.error(key, "is missing");
            return None;
        };
        read(value, self.path_of(key), self.errors)
    }

    /// `None` when the field isn't there, or has a mistake.
    pub fn optional_with<T>(&mut self, key: &str, read: Reader<T>) -> Option<T> {
        let value = self.table.get(key)?;
        read(value, self.path_of(key), self.errors)
    }

    /// A list of tables, keeping those that read cleanly. Empty when the field isn't there.
    pub fn list_with<T>(&mut self, key: &str, read: Reader<T>) -> Vec<T> {
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
    pub fn entries<T>(&mut self, key: &str, read: Reader<T>) -> Option<Entries<T>> {
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
