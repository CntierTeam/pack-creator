//! Item models, entity model/texture replacement, and GUI-related pack writes.

use crate::error::{Error, Result};
use crate::mapping::{cmd_allocator, IdAllocator};
use crate::project::Project;
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct ModelWriteStats {
    pub item_models: usize,
    pub modern_items: usize,
    pub legacy_override_files: usize,
    pub entity_models: usize,
    pub entity_texture_replacements: usize,
}

pub fn generate_item_models(
    project: &Project,
    items: &BTreeMap<String, YamlValue>,
    cache: &Path,
    staging: &Path,
) -> Result<ModelWriteStats> {
    let mut stats = ModelWriteStats::default();
    if items.is_empty() {
        return Ok(stats);
    }

    let mut cmd_allocs: BTreeMap<String, IdAllocator> = BTreeMap::new();
    // material -> list of override objects
    let mut legacy: BTreeMap<String, Vec<JsonValue>> = BTreeMap::new();
    // material -> fallback parent guess
    let mut legacy_parents: BTreeMap<String, String> = BTreeMap::new();

    for (id, value) in items {
        let Some(map) = value.as_mapping() else {
            continue;
        };

        let material = map
            .get(YamlValue::String("material".into()))
            .and_then(|v| v.as_str())
            .unwrap_or("paper")
            .to_ascii_lowercase();

        let model_map = map
            .get(YamlValue::String("model".into()))
            .and_then(|v| v.as_mapping());

        let model_path = resolve_model_path(id, map, model_map)?;
        let (ns, rel) = split_key(&model_path);

        // --- generation / texture → models/*.json ---
        if let Some(gen_json) = build_generation_json(map, model_map)? {
            let out = staging
                .join("assets")
                .join(&ns)
                .join("models")
                .join(format!("{rel}.json"));
            write_json(&out, &gen_json)?;
            stats.item_models += 1;
        }

        // --- CMD allocation ---
        let cmd = resolve_cmd(id, map, &material, project, cache, &mut cmd_allocs)?;

        // --- modern 1.21.4+ items/<id>.json ---
        let (id_ns, id_rel) = split_key(id);
        let modern = json!({
            "model": {
                "type": "model",
                "model": model_path
            }
        });
        let modern_out = staging
            .join("assets")
            .join(id_ns)
            .join("items")
            .join(format!("{id_rel}.json"));
        write_json(&modern_out, &modern)?;
        stats.modern_items += 1;

        // --- legacy CMD override entry ---
        legacy
            .entry(material.clone())
            .or_default()
            .push(json!({
                "predicate": { "custom_model_data": cmd },
                "model": model_path
            }));
        let parent_guess = guess_parent_for_material(&material);
        legacy_parents.entry(material).or_insert(parent_guess);
    }

    for alloc in cmd_allocs.values_mut() {
        alloc.process_pending();
        alloc.save()?;
    }

    // Sort overrides by CMD for stability
    for (material, overrides) in &mut legacy {
        overrides.sort_by(|a, b| {
            let ca = a["predicate"]["custom_model_data"].as_u64().unwrap_or(0);
            let cb = b["predicate"]["custom_model_data"].as_u64().unwrap_or(0);
            ca.cmp(&cb)
        });
        let parent = legacy_parents
            .get(material)
            .cloned()
            .unwrap_or_else(|| "minecraft:item/generated".into());
        let out = staging
            .join("assets")
            .join("minecraft")
            .join("models")
            .join("item")
            .join(format!("{material}.json"));

        let mut doc = if out.is_file() {
            serde_json::from_str(&fs::read_to_string(&out)?).unwrap_or_else(|_| {
                json!({ "parent": parent, "textures": { "layer0": format!("minecraft:item/{material}") } })
            })
        } else {
            json!({
                "parent": parent,
                "textures": { "layer0": format!("minecraft:item/{material}") }
            })
        };

        let existing = doc
            .as_object_mut()
            .ok_or_else(|| Error::Pack(format!("bad base model for {material}")))?;
        let mut merged = Vec::new();
        if let Some(JsonValue::Array(arr)) = existing.get("overrides") {
            merged.extend(arr.clone());
        }
        merged.extend(overrides.clone());
        existing.insert("overrides".into(), JsonValue::Array(merged));
        if !existing.contains_key("parent") {
            existing.insert("parent".into(), JsonValue::String(parent));
        }
        write_json(&out, &JsonValue::Object(existing.clone()))?;
        stats.legacy_override_files += 1;
    }

    Ok(stats)
}

pub fn generate_entity_models(
    project: &Project,
    entities: &BTreeMap<String, YamlValue>,
    staging: &Path,
) -> Result<ModelWriteStats> {
    let mut stats = ModelWriteStats::default();
    let rp_root = crate::project::Project::resourcepack_dir(&project.root);

    for (id, value) in entities {
        let Some(map) = value.as_mapping() else {
            continue;
        };

        // Optional model generation → assets/<ns>/models/entity/<path>.json
        if let Some(model) = map.get(YamlValue::String("model".into())) {
            let model_map = model.as_mapping();
            let path = model_map
                .and_then(|m| m.get(YamlValue::String("path".into())))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| {
                    let (ns, rel) = split_key(id);
                    format!("{ns}:entity/{rel}")
                });
            let (ns, rel) = split_key(&path);
            let gen = if let Some(m) = model_map {
                // Accept either nested generation: {} or parent/textures on the model map itself
                if let Some(gen) = m.get(YamlValue::String("generation".into())) {
                    gen.as_mapping()
                        .map(build_generation_from_map)
                        .transpose()?
                        .flatten()
                } else {
                    build_generation_from_map(m)?
                }
            } else {
                None
            };
            if let Some(json) = gen {
                let out = staging
                    .join("assets")
                    .join(ns)
                    .join("models")
                    .join(format!("{rel}.json"));
                write_json(&out, &json)?;
                stats.entity_models += 1;
            }
        }

        // replace_textures: copy from project resourcepack (or relative textures) into staging vanilla/custom paths
        if let Some(YamlValue::Sequence(list)) = map.get(YamlValue::String("replace_textures".into()))
        {
            for entry in list {
                let Some(em) = entry.as_mapping() else {
                    continue;
                };
                let from = em
                    .get(YamlValue::String("from".into()))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Pack(format!("entity {id}: replace_textures.from required")))?;
                let to = em
                    .get(YamlValue::String("to".into()))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Pack(format!("entity {id}: replace_textures.to required")))?;

                let src = resolve_texture_file(&rp_root, from)?;
                let dst = texture_asset_path(staging, to);
                if let Some(parent) = dst.parent() {
                    fs::create_dir_all(parent)?;
                }
                if src.is_file() {
                    fs::copy(&src, &dst)?;
                    stats.entity_texture_replacements += 1;
                } else {
                    return Err(Error::Pack(format!(
                        "entity {id}: texture not found: {} (from {from})",
                        src.display()
                    )));
                }
            }
        }
    }

    Ok(stats)
}

fn resolve_model_path(
    id: &str,
    item_map: &serde_yaml::Mapping,
    model_map: Option<&serde_yaml::Mapping>,
) -> Result<String> {
    if let Some(m) = model_map {
        if let Some(p) = m
            .get(YamlValue::String("path".into()))
            .and_then(|v| v.as_str())
        {
            return Ok(p.to_string());
        }
    }
    if let Some(t) = item_map
        .get(YamlValue::String("texture".into()))
        .and_then(|v| v.as_str())
    {
        // texture path used as model path convention: ns:item/foo
        return Ok(t.to_string());
    }
    let (ns, rel) = split_key(id);
    Ok(format!("{ns}:item/{rel}"))
}

fn build_generation_json(
    item_map: &serde_yaml::Mapping,
    model_map: Option<&serde_yaml::Mapping>,
) -> Result<Option<JsonValue>> {
    if let Some(m) = model_map {
        if let Some(gen) = m.get(YamlValue::String("generation".into())) {
            if let Some(gm) = gen.as_mapping() {
                return Ok(build_generation_from_map(gm)?);
            }
        }
        // model without generation but with type minecraft:model — still may need textures from item
    }

    // simplified: texture / textures
    if let Some(tex) = item_map
        .get(YamlValue::String("texture".into()))
        .and_then(|v| v.as_str())
    {
        return Ok(Some(json!({
            "parent": "minecraft:item/generated",
            "textures": { "layer0": tex }
        })));
    }
    if let Some(YamlValue::Sequence(layers)) = item_map.get(YamlValue::String("textures".into())) {
        let mut textures = JsonMap::new();
        for (i, layer) in layers.iter().enumerate() {
            if let Some(s) = layer.as_str() {
                textures.insert(format!("layer{i}"), JsonValue::String(s.to_string()));
            }
        }
        return Ok(Some(json!({
            "parent": "minecraft:item/generated",
            "textures": textures
        })));
    }

    // generation-only under model already handled; if model.path only, user must supply model file in resourcepack
    Ok(None)
}

fn build_generation_from_map(gm: &serde_yaml::Mapping) -> Result<Option<JsonValue>> {
    let parent = gm
        .get(YamlValue::String("parent".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("minecraft:item/generated");
    let mut obj = JsonMap::new();
    obj.insert("parent".into(), JsonValue::String(parent.to_string()));

    if let Some(tex) = gm.get(YamlValue::String("textures".into())) {
        match tex {
            YamlValue::Mapping(tm) => {
                let mut textures = JsonMap::new();
                for (k, v) in tm {
                    if let (Some(ks), Some(vs)) = (k.as_str(), v.as_str()) {
                        textures.insert(ks.to_string(), JsonValue::String(vs.to_string()));
                    }
                }
                obj.insert("textures".into(), JsonValue::Object(textures));
            }
            YamlValue::Sequence(seq) => {
                let mut textures = JsonMap::new();
                for (i, layer) in seq.iter().enumerate() {
                    if let Some(s) = layer.as_str() {
                        textures.insert(format!("layer{i}"), JsonValue::String(s.to_string()));
                    }
                }
                obj.insert("textures".into(), JsonValue::Object(textures));
            }
            _ => {}
        }
    }

    if let Some(gl) = gm
        .get(YamlValue::String("gui_light".into()))
        .or_else(|| gm.get(YamlValue::String("gui-light".into())))
        .and_then(|v| v.as_str())
    {
        obj.insert("gui_light".into(), JsonValue::String(gl.to_string()));
    }

    Ok(Some(JsonValue::Object(obj)))
}

fn resolve_cmd(
    id: &str,
    map: &serde_yaml::Mapping,
    material: &str,
    project: &Project,
    cache: &Path,
    allocs: &mut BTreeMap<String, IdAllocator>,
) -> Result<u32> {
    if let Some(v) = map
        .get(YamlValue::String("custom_model_data".into()))
        .or_else(|| map.get(YamlValue::String("custom-model-data".into())))
    {
        let cmd = match v {
            YamlValue::Number(n) => n.as_u64().unwrap_or(0) as u32,
            YamlValue::String(s) => s.parse().unwrap_or(0),
            _ => 0,
        };
        if !allocs.contains_key(material) {
            allocs.insert(
                material.to_string(),
                cmd_allocator(&project.build, cache, material)?,
            );
        }
        let alloc = allocs.get_mut(material).unwrap();
        alloc.assign_fixed(id, cmd)?;
        return Ok(cmd);
    }

    if !allocs.contains_key(material) {
        allocs.insert(
            material.to_string(),
            cmd_allocator(&project.build, cache, material)?,
        );
    }
    let alloc = allocs.get_mut(material).unwrap();
    Ok(alloc.request_auto(id))
}

fn guess_parent_for_material(material: &str) -> String {
    const HANDHELD: &[&str] = &[
        "wooden_sword",
        "stone_sword",
        "iron_sword",
        "golden_sword",
        "diamond_sword",
        "netherite_sword",
        "wooden_pickaxe",
        "stone_pickaxe",
        "iron_pickaxe",
        "golden_pickaxe",
        "diamond_pickaxe",
        "netherite_pickaxe",
        "wooden_axe",
        "stone_axe",
        "iron_axe",
        "golden_axe",
        "diamond_axe",
        "netherite_axe",
        "wooden_shovel",
        "stone_shovel",
        "iron_shovel",
        "golden_shovel",
        "diamond_shovel",
        "netherite_shovel",
        "wooden_hoe",
        "stone_hoe",
        "iron_hoe",
        "golden_hoe",
        "diamond_hoe",
        "netherite_hoe",
        "trident",
        "bow",
        "crossbow",
    ];
    if HANDHELD.contains(&material) {
        "minecraft:item/handheld".into()
    } else {
        "minecraft:item/generated".into()
    }
}

fn split_key(key: &str) -> (String, String) {
    if let Some((a, b)) = key.split_once(':') {
        (a.to_string(), b.to_string())
    } else {
        ("minecraft".into(), key.to_string())
    }
}

fn write_json(path: &Path, value: &JsonValue) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn resolve_texture_file(rp_root: &Path, key: &str) -> Result<PathBuf> {
    // key like ns:entity/foo or ns:entity/foo.png → assets/ns/textures/entity/foo.png
    let key = key.strip_suffix(".png").unwrap_or(key);
    let (ns, rel) = split_key(key);
    Ok(rp_root
        .join("assets")
        .join(ns)
        .join("textures")
        .join(format!("{rel}.png")))
}

fn texture_asset_path(staging: &Path, key: &str) -> PathBuf {
    let key = key.strip_suffix(".png").unwrap_or(key);
    let (ns, rel) = split_key(key);
    staging
        .join("assets")
        .join(ns)
        .join("textures")
        .join(format!("{rel}.png"))
}
