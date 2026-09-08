use crate::error::{Error, Result};
use serde::Serialize;
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Configuration section roots recognized by PackCreator.
pub const SECTION_ALIASES: &[(&str, &str)] = &[
    ("templates", "templates"),
    ("template", "templates"),
    ("global-variables", "global-variables"),
    ("global_variables", "global-variables"),
    ("global_variable", "global-variables"),
    ("images", "images"),
    ("image", "images"),
    ("emojis", "emojis"),
    ("emoji", "emojis"),
    ("lang", "lang"),
    ("language", "lang"),
    ("languages", "lang"),
    ("translations", "translations"),
    ("translation", "translations"),
    ("l10n", "translations"),
    ("i18n", "translations"),
    ("sounds", "sounds"),
    ("sound", "sounds"),
    ("jukebox-songs", "jukebox-songs"),
    ("jukebox_songs", "jukebox-songs"),
    ("jukebox_song", "jukebox-songs"),
    ("equipments", "equipments"),
    ("equipment", "equipments"),
    ("items", "items"),
    ("item", "items"),
    ("blocks", "blocks"),
    ("block", "blocks"),
    ("block_state_mappings", "block_state_mappings"),
    ("block-state-mappings", "block_state_mappings"),
    ("furniture", "furniture"),
    ("paintings", "paintings"),
    ("painting", "paintings"),
    ("recipes", "recipes"),
    ("recipe", "recipes"),
    ("categories", "categories"),
    ("category", "categories"),
    ("loot-tables", "loot-tables"),
    ("loot_tables", "loot-tables"),
    ("vanilla-loots", "vanilla-loots"),
    ("config_factory", "config_factory"),
    ("config-factories", "config_factory"),
];

#[derive(Debug, Clone, Default, Serialize)]
pub struct ConfigIndex {
    pub files: Vec<PathBuf>,
    pub sections: BTreeMap<String, Vec<SectionHit>>,
    pub unknown_roots: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionHit {
    pub file: PathBuf,
    pub root_key: String,
    pub canonical: String,
}

#[derive(Debug, Clone, Default)]
pub struct LoadedConfigs {
    pub images: BTreeMap<String, Value>,
    pub emojis: BTreeMap<String, Value>,
    pub langs: BTreeMap<String, BTreeMap<String, String>>,
    pub sounds: BTreeMap<String, Value>,
    pub equipments: BTreeMap<String, Value>,
    pub raw_sections: BTreeMap<String, Vec<(PathBuf, Value)>>,
}

pub fn scan_configuration(dir: &Path) -> Result<ConfigIndex> {
    let mut index = ConfigIndex::default();
    if !dir.is_dir() {
        return Ok(index);
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "yml" && ext != "yaml" && ext != "json" {
            continue;
        }
        index.files.push(path.to_path_buf());
        let text = fs::read_to_string(path)?;
        let value: Value = if ext == "json" {
            serde_json::from_str(&text).map_err(|e| Error::Msg(e.to_string()))?
        } else {
            serde_yaml::from_str(&text)?
        };
        let Value::Mapping(map) = value else {
            continue;
        };
        for key in map.keys() {
            let Some(k) = key.as_str() else {
                continue;
            };
            let base = k.split('#').next().unwrap_or(k);
            if let Some((_, canon)) = SECTION_ALIASES.iter().find(|(a, _)| *a == base) {
                index
                    .sections
                    .entry((*canon).to_string())
                    .or_default()
                    .push(SectionHit {
                        file: path.to_path_buf(),
                        root_key: k.to_string(),
                        canonical: (*canon).to_string(),
                    });
            } else {
                index.unknown_roots.insert(base.to_string());
            }
        }
    }
    Ok(index)
}

pub fn load_configs(dir: &Path) -> Result<LoadedConfigs> {
    let mut loaded = LoadedConfigs::default();
    if !dir.is_dir() {
        return Ok(loaded);
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "yml" && ext != "yaml" {
            continue;
        }
        let text = fs::read_to_string(path)?;
        let value: Value = serde_yaml::from_str(&text)?;
        let Value::Mapping(map) = value else {
            continue;
        };
        for (key, val) in map {
            let Some(k) = key.as_str() else {
                continue;
            };
            let base = k.split('#').next().unwrap_or(k);
            let canon = SECTION_ALIASES
                .iter()
                .find(|(a, _)| *a == base)
                .map(|(_, c)| *c)
                .unwrap_or(base);
            loaded
                .raw_sections
                .entry(canon.to_string())
                .or_default()
                .push((path.to_path_buf(), val.clone()));

            match canon {
                "images" => {
                    if let Value::Mapping(m) = val {
                        for (ik, iv) in m {
                            if let Some(id) = ik.as_str() {
                                loaded.images.insert(id.to_string(), iv);
                            }
                        }
                    }
                }
                "emojis" => {
                    if let Value::Mapping(m) = val {
                        for (ik, iv) in m {
                            if let Some(id) = ik.as_str() {
                                loaded.emojis.insert(id.to_string(), iv);
                            }
                        }
                    }
                }
                "lang" => {
                    if let Value::Mapping(locales) = val {
                        for (lk, lv) in locales {
                            let Some(locale) = lk.as_str() else {
                                continue;
                            };
                            let entry = loaded.langs.entry(locale.to_string()).or_default();
                            if let Value::Mapping(pairs) = lv {
                                for (pk, pv) in pairs {
                                    if let (Some(p), Some(s)) = (pk.as_str(), yaml_to_string(&pv))
                                    {
                                        entry.insert(p.to_string(), s);
                                    }
                                }
                            }
                        }
                    }
                }
                "sounds" => {
                    if let Value::Mapping(m) = val {
                        for (ik, iv) in m {
                            if let Some(id) = ik.as_str() {
                                loaded.sounds.insert(id.to_string(), iv);
                            }
                        }
                    }
                }
                "equipments" => {
                    if let Value::Mapping(m) = val {
                        for (ik, iv) in m {
                            if let Some(id) = ik.as_str() {
                                loaded.equipments.insert(id.to_string(), iv);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(loaded)
}

fn yaml_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_conf() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("pc-conf-{nanos}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scan_recognizes_aliases_and_hash_suffix() {
        let dir = tmp_conf();
        fs::write(
            dir.join("a.yml"),
            "image:\n  ns:icon: { file: ns:x.png }\nlang#items:\n  en_us:\n    k: v\n",
        )
        .unwrap();
        fs::write(dir.join("b.yml"), "unknown_root:\n  x: 1\n").unwrap();
        let idx = scan_configuration(&dir).unwrap();
        assert!(idx.sections.contains_key("images"));
        assert!(idx.sections.contains_key("lang"));
        assert_eq!(idx.sections["lang"][0].root_key, "lang#items");
        assert!(idx.unknown_roots.contains("unknown_root"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_merges_lang_and_images() {
        let dir = tmp_conf();
        fs::write(
            dir.join("images.yml"),
            "images:\n  a:icon:\n    file: a:icon.png\n    height: 8\n",
        )
        .unwrap();
        fs::write(
            dir.join("lang.yml"),
            "lang:\n  en_us:\n    hello: world\n  zh_cn:\n    hello: 你好\n",
        )
        .unwrap();
        let loaded = load_configs(&dir).unwrap();
        assert!(loaded.images.contains_key("a:icon"));
        assert_eq!(loaded.langs["en_us"]["hello"], "world");
        assert_eq!(loaded.langs["zh_cn"]["hello"], "你好");
        let _ = fs::remove_dir_all(dir);
    }
}
