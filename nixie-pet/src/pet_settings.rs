//! 宠物外观与偏好：`~/.nixie/pet_settings.json`（与 overlay 分离，便于日后多物种扩展）。

use serde::{Deserialize, Serialize};

use crate::window_prefs::nixie_data_dir;

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PetSettings {
    #[serde(default = "schema_v1")]
    pub schema_version: u32,
    /// 展示用名字（未来可用于气泡署名等）
    #[serde(default = "default_name")]
    pub name: String,
    /// `normal` | `small` | `mini` → 1.0 / 0.8 / 0.5 仅缩放猪身与拖尾，不改台词与 Rust 判定区
    #[serde(default)]
    pub body_size: String,
    /// UI 语言：`zh` | `en` | `ja` | `binary`（binary 走 zh 文案，仅作趣味标签）
    #[serde(default = "default_locale")]
    pub locale: String,
    /// 物种键，预留多动物：`virtual_pig` 等
    #[serde(default = "default_breed")]
    pub breed: String,
}

fn schema_v1() -> u32 {
    SCHEMA_VERSION
}

fn default_name() -> String {
    "OINKER_01".to_string()
}

fn default_locale() -> String {
    "zh".to_string()
}

fn default_breed() -> String {
    "virtual_pig".to_string()
}

impl Default for PetSettings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: default_name(),
            body_size: "normal".to_string(),
            locale: default_locale(),
            breed: default_breed(),
        }
    }
}

impl PetSettings {
    pub fn body_scale(&self) -> f64 {
        match self.body_size.as_str() {
            "small" => 0.8,
            "mini" => 0.5,
            _ => 1.0,
        }
    }

    /// `binary` 仅作彩蛋，运行时仍用 zh 包
    pub fn effective_locale(&self) -> &str {
        match self.locale.as_str() {
            "en" => "en",
            "ja" => "ja",
            "binary" | "zh" | _ => "zh",
        }
    }

    pub fn sanitized(mut self) -> Self {
        self.name = self.name.trim().chars().take(64).collect();
        if self.name.is_empty() {
            self.name = default_name();
        }
        let bs = self.body_size.to_lowercase();
        self.body_size = match bs.as_str() {
            "small" | "mini" | "normal" => bs,
            _ => "normal".to_string(),
        };
        let loc = self.locale.to_lowercase();
        self.locale = match loc.as_str() {
            "en" | "ja" | "binary" | "zh" => loc,
            _ => "zh".to_string(),
        };
        let br = self.breed.to_lowercase();
        self.breed = if br.is_empty() {
            default_breed()
        } else {
            br.chars().take(48).collect()
        };
        self
    }
}

fn settings_path() -> std::path::PathBuf {
    nixie_data_dir().join("pet_settings.json")
}

pub fn load_pet_settings() -> PetSettings {
    let path = settings_path();
    let s = std::fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str::<PetSettings>(&s)
        .unwrap_or_default()
        .sanitized()
}

pub fn save_pet_settings(settings: &PetSettings) -> bool {
    let path = settings_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let s = settings.clone().sanitized();
    serde_json::to_string_pretty(&s)
        .ok()
        .and_then(|json| std::fs::write(&path, json).ok())
        .is_some()
}

#[derive(Debug, Deserialize)]
pub struct PetSettingsPatch {
    pub name: String,
    #[serde(rename = "bodySize")]
    pub body_size: String,
    pub locale: String,
    pub breed: String,
}

/// 供 WebView 注入：`bodyScale`、`effectiveLocale` 为派生字段。
pub fn to_web_object_json(s: &PetSettings) -> String {
    serde_json::json!({
        "name": s.name,
        "bodySize": s.body_size,
        "locale": s.locale,
        "breed": s.breed,
        "bodyScale": s.body_scale(),
        "effectiveLocale": s.effective_locale(),
    })
    .to_string()
}

pub fn save_from_form_json(raw: &str) -> Result<PetSettings, ()> {
    let patch: PetSettingsPatch = serde_json::from_str(raw).map_err(|_| ())?;
    let mut s = load_pet_settings();
    s.name = patch.name;
    s.body_size = patch.body_size;
    s.locale = patch.locale;
    s.breed = patch.breed;
    s.schema_version = SCHEMA_VERSION;
    let s = s.sanitized();
    if save_pet_settings(&s) {
        Ok(s)
    } else {
        Err(())
    }
}
