//! Editor preferences shared with 1.x (`preferences` state record).

use crate::document::PrintSettings;
use crate::settings::PRINT_CONTROLS;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GridAppearance {
    #[default]
    Flat,
    Embossed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedPrintPreset {
    pub id: String,
    pub name: String,
    pub settings: PrintSettings,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub version: u32,
    pub grid_appearance: GridAppearance,
    pub print_presets: Vec<SavedPrintPreset>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self { version: 1, grid_appearance: GridAppearance::Flat, print_presets: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct PreferencesError(pub String);

pub fn valid_print_settings(value: &Value) -> Option<PrintSettings> {
    let object = value.as_object()?;
    let mut settings = crate::settings::PRINT_DEFAULTS;
    for c in &PRINT_CONTROLS {
        let v = object.get(c.id)?.as_f64().filter(|v| v.is_finite() && *v >= c.min && *v <= c.max)?;
        *settings.get_mut(c.key) = v;
    }
    Some(settings)
}

/// Reads legacy appearance strings and records, keeping only supported fields.
pub fn parse_preferences(text: &str) -> Result<Preferences, PreferencesError> {
    let invalid = || PreferencesError("Invalid editor preferences.".into());
    match text {
        "flat" => return Ok(Preferences::default()),
        "embossed" => return Ok(Preferences { grid_appearance: GridAppearance::Embossed, ..Default::default() }),
        _ => {}
    }
    let value: Value = serde_json::from_str(text).map_err(|_| invalid())?;
    let object = value.as_object().ok_or_else(invalid)?;
    let grid_appearance: GridAppearance =
        serde_json::from_value(object.get("gridAppearance").cloned().unwrap_or(Value::Null)).map_err(|_| invalid())?;
    if !object.contains_key("version") {
        return Ok(Preferences { grid_appearance, ..Default::default() });
    }
    let rows = object.get("printPresets").and_then(Value::as_array);
    let Some(rows) = rows.filter(|r| r.len() <= 100 && object["version"].as_i64() == Some(1)) else {
        return Err(invalid());
    };
    let bad = || PreferencesError("Invalid saved presets.".into());
    let mut ids = HashSet::new();
    let mut presets = Vec::with_capacity(rows.len());
    for row in rows {
        let id = row.get("id").and_then(Value::as_str).filter(|id| (1..=100).contains(&id.len()));
        let name = row.get("name").and_then(Value::as_str).filter(|n| !n.trim().is_empty() && n.chars().count() <= 80);
        let settings = row.get("settings").and_then(valid_print_settings);
        let (Some(id), Some(name), Some(settings)) = (id, name, settings) else { return Err(bad()) };
        if !ids.insert(id.to_string()) {
            return Err(bad());
        }
        presets.push(SavedPrintPreset { id: id.into(), name: name.into(), settings });
    }
    Ok(Preferences { version: 1, grid_appearance, print_presets: presets })
}

pub fn serialize_preferences(p: &Preferences) -> String {
    serde_json::to_string(p).expect("preferences always serialize")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::PRINT_DEFAULTS;
    use serde_json::json;

    #[test]
    fn migrates_appearance_strings_and_original_records() {
        assert_eq!(parse_preferences("embossed").unwrap().grid_appearance, GridAppearance::Embossed);
        assert_eq!(parse_preferences(r#"{"gridAppearance":"flat"}"#).unwrap(), Preferences::default());
    }

    #[test]
    fn drops_unsupported_fields_but_keeps_print_settings() {
        let p = Preferences {
            print_presets: vec![SavedPrintPreset { id: "p".into(), name: "Cotton".into(), settings: PRINT_DEFAULTS }],
            ..Default::default()
        };
        let mut value = serde_json::to_value(&p).unwrap();
        value["unused"] = json!({ "custom": true });
        assert_eq!(parse_preferences(&value.to_string()).unwrap(), p);
        let written: Value = serde_json::from_str(&serialize_preferences(&p)).unwrap();
        assert_eq!(written.as_object().unwrap().len(), 3);
    }

    #[test]
    fn rejects_invalid_controls_names_ids_and_versions() {
        let valid = json!({ "id": "p", "name": "Cotton", "settings": PRINT_DEFAULTS });
        let mut loud = valid.clone();
        loud["settings"]["inkAmount"] = 4.into();
        let mut blank = valid.clone();
        blank["name"] = " ".into();
        let mut other = valid.clone();
        other["name"] = "Other".into();
        for rows in [vec![loud], vec![blank], vec![valid.clone(), other]] {
            let text = json!({ "version": 1, "gridAppearance": "flat", "printPresets": rows }).to_string();
            assert!(parse_preferences(&text).is_err());
        }
        let text = json!({ "version": 2, "gridAppearance": "flat", "printPresets": [] }).to_string();
        assert!(parse_preferences(&text).is_err());
    }
}
