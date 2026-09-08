use crate::config::{load_configs, scan_configuration, LoadedConfigs};
use crate::error::{Error, Result};
use crate::mapping::{
    embedded_whole_mappings_yaml, font_allocator, prepare_mappings, IdAllocator,
};
use crate::project::{PackYml, Project, ResolvedPackTarget, ZipConfig, ZipMethod};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;
use zip::ZipWriter;

mod models;

#[derive(Debug, Clone)]
pub struct BuildReport {
    pub pack_dir: PathBuf,
    pub resource_pack_zip: PathBuf,
    pub resource_pack_zips: Vec<PathBuf>,
    pub variants_built: Vec<String>,
    pub sections: Vec<String>,
    pub fonts_written: usize,
    pub langs_written: usize,
    pub sounds_written: usize,
    pub files_copied: usize,
    pub item_models: usize,
    pub modern_items: usize,
    pub legacy_override_files: usize,
    pub entity_models: usize,
    pub entity_texture_replacements: usize,
}

pub fn build_project(project: &Project) -> Result<BuildReport> {
    build_project_filtered(project, &[])
}

pub fn build_project_filtered(project: &Project, variant_filter: &[String]) -> Result<BuildReport> {
    let conf_dir = Project::configuration_dir(&project.root);
    let mut index = scan_configuration(&conf_dir)?;
    let mut configs = load_configs(&conf_dir)?;
    // build.pk is the aggregation source of truth (wins over YAML overlays)
    project.build.apply_to_configs(&mut configs);
    for section in project.build.contents.keys() {
        index
            .sections
            .entry(section.clone())
            .or_default()
            .push(crate::config::SectionHit {
                file: Project::build_pk_path(&project.root),
                root_key: section.clone(),
                canonical: section.clone(),
            });
    }
    let cache = project.cache_dir();
    fs::create_dir_all(&cache)?;
    let _mappings = prepare_mappings(&project.build, &cache)?;

    let targets = project.build.resolve_pack_targets(variant_filter)?;
    let pack_out = project.root.join(&project.build.export.pack_dir);
    let pack_name = &project.build.project.name;
    let pack_root = pack_out.join(pack_name);
    export_pack_tree(project, &pack_root, &configs)?;

    let staging = project.root.join("build").join("staging_rp");
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let mut report = BuildReport {
        pack_dir: pack_root.clone(),
        resource_pack_zip: PathBuf::new(),
        resource_pack_zips: Vec::new(),
        variants_built: Vec::new(),
        sections: index.sections.keys().cloned().collect(),
        fonts_written: 0,
        langs_written: 0,
        sounds_written: 0,
        files_copied: 0,
        item_models: 0,
        modern_items: 0,
        legacy_override_files: 0,
        entity_models: 0,
        entity_texture_replacements: 0,
    };

    // Base resourcepack without overlays/ (merged per variant)
    report.files_copied = copy_dir_merge_skip(
        &Project::resourcepack_dir(&project.root),
        &staging,
        &["overlays"],
    )?;

    if project.build.pack.features.images || project.build.pack.features.gui {
        report.fonts_written =
            generate_fonts(project, &configs, &cache, &staging)?;
    }
    if project.build.pack.features.lang {
        report.langs_written = generate_langs(&configs, &staging)?;
    }
    if project.build.pack.features.sounds {
        report.sounds_written = generate_sounds(&configs, &staging)?;
    }
    if project.build.pack.features.equipment {
        generate_equipments(&configs, &staging)?;
    }
    if project.build.pack.features.items {
        let item_stats =
            models::generate_item_models(project, &configs.items, &cache, &staging)?;
        report.item_models = item_stats.item_models;
        report.modern_items = item_stats.modern_items;
        report.legacy_override_files = item_stats.legacy_override_files;
    }
    if project.build.pack.features.entities {
        let ent_stats =
            models::generate_entity_models(project, &configs.entity_models, &staging)?;
        report.entity_models = ent_stats.entity_models;
        report.entity_texture_replacements = ent_stats.entity_texture_replacements;
    }

    for target in &targets {
        let zip_path = project.root.join(&target.resource_pack_zip);
        let variant_staging = if targets.len() == 1 && target.overlays.is_empty() {
            write_pack_mcmeta_for_target(project, target, &staging)?;
            staging.clone()
        } else {
            let vs = project
                .root
                .join("build")
                .join(format!("staging_rp_{}", target.name));
            if vs.exists() {
                fs::remove_dir_all(&vs)?;
            }
            copy_dir_merge(&staging, &vs)?;
            merge_variant_overlays(project, target, &vs)?;
            write_pack_mcmeta_for_target(project, target, &vs)?;
            vs
        };

        if let Some(parent) = zip_path.parent() {
            fs::create_dir_all(parent)?;
        }
        zip_directory(&variant_staging, &zip_path, &project.build.zip)?;
        report.resource_pack_zips.push(zip_path);
        report.variants_built.push(target.name.clone());
    }

    report.resource_pack_zip = report
        .resource_pack_zips
        .first()
        .cloned()
        .unwrap_or_else(|| project.root.join(&project.build.export.resource_pack_zip));

    // Keep base staging pack.mcmeta aligned with first/default target for pack tree consumers
    if let Some(first) = targets.first() {
        write_pack_mcmeta_for_target(project, first, &staging)?;
    }

    let summary = json!({
        "project": pack_name,
        "namespace": project.build.project.namespace,
        "pack_dir": report.pack_dir,
        "resource_pack_zip": report.resource_pack_zip,
        "resource_pack_zips": report.resource_pack_zips,
        "variants_built": report.variants_built,
        "zip": {
            "method": match project.build.zip.method {
                ZipMethod::Deflated => "DEFLATED",
                ZipMethod::Stored => "STORED",
            },
            "level": project.build.zip.level.min(9),
        },
        "sections": report.sections,
        "fonts_written": report.fonts_written,
        "langs_written": report.langs_written,
        "sounds_written": report.sounds_written,
        "files_copied": report.files_copied,
        "item_models": report.item_models,
        "modern_items": report.modern_items,
        "legacy_override_files": report.legacy_override_files,
        "entity_models": report.entity_models,
        "entity_texture_replacements": report.entity_texture_replacements,
        "mappings_mode": match project.build.mappings.mode {
            crate::project::MappingsMode::Whole => "WHOLE",
            crate::project::MappingsMode::Custom => "CUSTOM",
        }
    });
    fs::write(
        project.root.join("build").join("report.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;

    Ok(report)
}

fn merge_variant_overlays(
    project: &Project,
    target: &ResolvedPackTarget,
    staging: &Path,
) -> Result<()> {
    let overlays_root = Project::resourcepack_dir(&project.root).join("overlays");
    for name in &target.overlays {
        let src = overlays_root.join(name);
        if src.is_dir() {
            copy_dir_merge(&src, staging)?;
        }
    }
    Ok(())
}

fn export_pack_tree(
    project: &Project,
    dest: &Path,
    configs: &LoadedConfigs,
) -> Result<()> {
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::create_dir_all(dest)?;

    let pack: PackYml = (&project.build).into();
    fs::write(dest.join("pack.yml"), serde_yaml::to_string(&pack)?)?;

    // configuration: optional YAML overlays, then emit aggregated build.pk contents
    let conf_src = Project::configuration_dir(&project.root);
    let conf_dst = dest.join("configuration");
    copy_dir_merge(&conf_src, &conf_dst)?;
    emit_build_pk_configuration(project, &conf_dst)?;

    if project.build.mappings.mode == crate::project::MappingsMode::Whole {
        // Emit full block_state_mappings unless user already provided one
        let has_mappings = configs
            .raw_sections
            .contains_key("block_state_mappings");
        if !has_mappings {
            fs::write(
                conf_dst.join("block_state_mappings.yml"),
                embedded_whole_mappings_yaml(),
            )?;
        }
    }

    // Emit allocator seeds for tooling / future packers
    let mut mapping_meta = serde_yaml::Mapping::new();
    mapping_meta.insert(
        YamlValue::String("mode".into()),
        YamlValue::String(match project.build.mappings.mode {
            crate::project::MappingsMode::Whole => "WHOLE".into(),
            crate::project::MappingsMode::Custom => "CUSTOM".into(),
        }),
    );
    mapping_meta.insert(
        YamlValue::String("font".into()),
        serde_yaml::to_value(&project.build.mappings.font)?,
    );
    mapping_meta.insert(
        YamlValue::String("custom_model_data".into()),
        serde_yaml::to_value(&project.build.mappings.custom_model_data)?,
    );
    fs::write(
        conf_dst.join("_pack_creator_mappings.yml"),
        serde_yaml::to_string(&YamlValue::Mapping({
            let mut m = serde_yaml::Mapping::new();
            m.insert(
                YamlValue::String("pack_creator_mappings".into()),
                YamlValue::Mapping(mapping_meta),
            );
            m
        }))?,
    )?;

    let rp_src = Project::resourcepack_dir(&project.root);
    let rp_dst = dest.join("resourcepack");
    copy_dir_merge(&rp_src, &rp_dst)?;

    Ok(())
}

fn emit_build_pk_configuration(project: &Project, conf_dst: &Path) -> Result<()> {
    fs::create_dir_all(conf_dst)?;
    for (section, entries) in &project.build.contents {
        let file_stem = match section.as_str() {
            "global-variables" => "global_variables",
            "loot-tables" => "loot_tables",
            other => other,
        };
        let mut body = serde_yaml::Mapping::new();
        for (id, v) in entries {
            body.insert(YamlValue::String(id.clone()), v.clone());
        }
        let mut root = serde_yaml::Mapping::new();
        root.insert(
            YamlValue::String(section.clone()),
            YamlValue::Mapping(body),
        );
        fs::write(
            conf_dst.join(format!("{file_stem}.yml")),
            serde_yaml::to_string(&YamlValue::Mapping(root))?,
        )?;
    }
    Ok(())
}

fn copy_dir_merge(src: &Path, dst: &Path) -> Result<usize> {
    copy_dir_merge_skip(src, dst, &[])
}

fn copy_dir_merge_skip(src: &Path, dst: &Path, skip_top: &[&str]) -> Result<usize> {
    if !src.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let rel = path.strip_prefix(src).unwrap();
        if let Some(first) = rel.components().next() {
            let name = first.as_os_str().to_string_lossy();
            if skip_top.iter().any(|s| *s == name) {
                continue;
            }
        }
        let target = dst.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, &target)?;
        count += 1;
    }
    Ok(count)
}

fn generate_fonts(
    project: &Project,
    configs: &LoadedConfigs,
    cache: &Path,
    staging: &Path,
) -> Result<usize> {
    // font key -> providers
    let mut fonts: BTreeMap<String, Vec<JsonValue>> = BTreeMap::new();
    let mut allocators: BTreeMap<String, IdAllocator> = BTreeMap::new();
    let ns = &project.build.project.namespace;

    // optional offset characters from embedded asset
    if project.build.mappings.font.offset_characters {
        inject_offset_chars(project, &mut fonts)?;
    }

    for (id, value) in &configs.images {
        let map = value.as_mapping().ok_or_else(|| {
            Error::Pack(format!("image {id} must be a mapping"))
        })?;
        let font = map
            .get(YamlValue::String("font".into()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{ns}:default"));
        let file = map
            .get(YamlValue::String("file".into()))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Pack(format!("image {id} missing file")))?;
        let height = map
            .get(YamlValue::String("height".into()))
            .and_then(|v| v.as_i64())
            .unwrap_or(16) as i32;
        let ascent = map
            .get(YamlValue::String("ascent".into()))
            .and_then(|v| v.as_i64())
            .unwrap_or((height - 1) as i64) as i32;

        if !allocators.contains_key(&font) {
            allocators.insert(font.clone(), font_allocator(&project.build, cache, &font)?);
        }
        let alloc = allocators.get_mut(&font).expect("font allocator inserted");

        let chars = resolve_chars(id, map, alloc)?;
        let mut provider = JsonMap::new();
        provider.insert("type".into(), json!("bitmap"));
        provider.insert("file".into(), json!(ensure_png(file)));
        provider.insert("height".into(), json!(height));
        provider.insert("ascent".into(), json!(ascent));
        provider.insert("chars".into(), JsonValue::Array(chars));
        fonts.entry(font).or_default().push(JsonValue::Object(provider));
    }

    for (font, alloc) in allocators.iter_mut() {
        let _ = font;
        alloc.process_pending();
        alloc.save()?;
    }

    let mut written = 0;
    for (font, providers) in fonts {
        let (f_ns, f_path) = split_key(&font);
        let out = staging
            .join("assets")
            .join(f_ns)
            .join("font")
            .join(format!("{f_path}.json"));
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut existing_providers = Vec::new();
        if out.is_file() {
            let text = fs::read_to_string(&out)?;
            if let Ok(JsonValue::Object(obj)) = serde_json::from_str::<JsonValue>(&text) {
                if let Some(JsonValue::Array(arr)) = obj.get("providers") {
                    existing_providers = arr.clone();
                }
            }
        }
        existing_providers.extend(providers);
        let doc = json!({ "providers": existing_providers });
        fs::write(&out, serde_json::to_string_pretty(&doc)?)?;
        written += 1;
    }
    Ok(written)
}

fn inject_offset_chars(
    project: &Project,
    fonts: &mut BTreeMap<String, Vec<JsonValue>>,
) -> Result<()> {
    let font = project.build.mappings.font.offset_font.clone();
    let yaml = include_str!("../../assets/offset_chars.yml");
    let root: YamlValue = serde_yaml::from_str(yaml)?;
    let Some(images) = root
        .as_mapping()
        .and_then(|m| m.get(YamlValue::String("images".into())))
        .and_then(|v| v.as_mapping())
    else {
        return Ok(());
    };
    for (_id, value) in images {
        let Some(map) = value.as_mapping() else {
            continue;
        };
        let file = map
            .get(YamlValue::String("file".into()))
            .and_then(|v| v.as_str())
            .unwrap_or("minecraft:font/offset/space_split.png");
        let height = map
            .get(YamlValue::String("height".into()))
            .and_then(|v| v.as_i64())
            .unwrap_or(-3) as i32;
        let ascent = map
            .get(YamlValue::String("ascent".into()))
            .and_then(|v| v.as_i64())
            .unwrap_or(-5000) as i32;
        let ch = map
            .get(YamlValue::String("char".into()))
            .and_then(|v| v.as_str())
            .unwrap_or("\u{f800}");
        let decoded = decode_char_token(ch);
        fonts.entry(font.clone()).or_default().push(json!({
            "type": "bitmap",
            "file": ensure_png(file),
            "height": height,
            "ascent": ascent,
            "chars": [decoded]
        }));
    }
    Ok(())
}

fn resolve_chars(
    id: &str,
    map: &serde_yaml::Mapping,
    alloc: &mut IdAllocator,
) -> Result<Vec<JsonValue>> {
    if let Some(v) = map
        .get(YamlValue::String("char".into()))
        .or_else(|| map.get(YamlValue::String("unicode".into())))
    {
        let s = yaml_scalar_string(v)?;
        let decoded = decode_char_token(&s);
        let cp = decoded.chars().next().map(|c| c as u32).unwrap_or(0);
        alloc.assign_fixed(id, cp)?;
        return Ok(vec![JsonValue::String(decoded)]);
    }
    if let Some(v) = map.get(YamlValue::String("chars".into())) {
        match v {
            YamlValue::Sequence(seq) => {
                let mut out = Vec::new();
                for (i, item) in seq.iter().enumerate() {
                    let s = yaml_scalar_string(item)?;
                    let decoded = decode_char_token(&s);
                    let cp = decoded.chars().next().map(|c| c as u32).unwrap_or(0);
                    alloc.assign_fixed(&format!("{id}:{i}"), cp)?;
                    out.push(JsonValue::String(decoded));
                }
                return Ok(out);
            }
            YamlValue::String(s) => {
                let decoded = decode_char_token(s);
                let cp = decoded.chars().next().map(|c| c as u32).unwrap_or(0);
                alloc.assign_fixed(id, cp)?;
                return Ok(vec![JsonValue::String(decoded)]);
            }
            _ => {}
        }
    }

    let (rows, cols) = parse_grid(
        map.get(YamlValue::String("grid_size".into()))
            .and_then(|v| v.as_str())
            .unwrap_or("1,1"),
    )?;
    let mut lines = Vec::new();
    for r in 0..rows {
        let mut line = String::new();
        for c in 0..cols {
            let key = format!("{id}:{r}:{c}");
            let cp = alloc.request_auto(&key);
            line.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
        }
        lines.push(JsonValue::String(line));
    }
    Ok(lines)
}

fn parse_grid(s: &str) -> Result<(usize, usize)> {
    let parts: Vec<_> = s.split(',').map(|p| p.trim()).collect();
    if parts.len() != 2 {
        return Err(Error::Pack(format!("bad grid_size: {s}")));
    }
    Ok((parts[0].parse()?, parts[1].parse()?))
}

fn decode_char_token(s: &str) -> String {
    // supports \uf800 and raw chars
    if let Some(hex) = s.strip_prefix("\\u").or_else(|| s.strip_prefix("\\U")) {
        if let Ok(cp) = u32::from_str_radix(hex, 16) {
            if let Some(ch) = char::from_u32(cp) {
                return ch.to_string();
            }
        }
    }
    if s.chars().count() == 1 {
        return s.to_string();
    }
    // try unicode escape without backslash: uf800 / U+F800
    if let Some(hex) = s.strip_prefix('u').or_else(|| s.strip_prefix("U+")) {
        if let Ok(cp) = u32::from_str_radix(hex, 16) {
            if let Some(ch) = char::from_u32(cp) {
                return ch.to_string();
            }
        }
    }
    s.to_string()
}

fn yaml_scalar_string(v: &YamlValue) -> Result<String> {
    match v {
        YamlValue::String(s) => Ok(s.clone()),
        YamlValue::Number(n) => Ok(n.to_string()),
        _ => Err(Error::Pack("expected scalar".into())),
    }
}

fn ensure_png(file: &str) -> String {
    if file.ends_with(".png") {
        file.to_string()
    } else {
        format!("{file}.png")
    }
}

fn split_key(key: &str) -> (String, String) {
    if let Some((a, b)) = key.split_once(':') {
        (a.to_string(), b.to_string())
    } else {
        ("minecraft".into(), key.to_string())
    }
}

fn generate_langs(configs: &LoadedConfigs, staging: &Path) -> Result<usize> {
    let mut written = 0;
    for (locale, pairs) in &configs.langs {
        let out = staging
            .join("assets")
            .join("minecraft")
            .join("lang")
            .join(format!("{locale}.json"));
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut map: BTreeMap<String, String> = BTreeMap::new();
        if out.is_file() {
            let text = fs::read_to_string(&out)?;
            if let Ok(existing) = serde_json::from_str::<BTreeMap<String, String>>(&text) {
                map.extend(existing);
            }
        }
        map.extend(pairs.clone());
        fs::write(&out, serde_json::to_string_pretty(&map)?)?;
        written += 1;
    }
    Ok(written)
}

fn generate_sounds(configs: &LoadedConfigs, staging: &Path) -> Result<usize> {
    // group by namespace
    let mut by_ns: BTreeMap<String, JsonMap<String, JsonValue>> = BTreeMap::new();
    for (id, value) in &configs.sounds {
        let (ns, path) = split_key(id);
        let json = yaml_to_json(value)?;
        by_ns.entry(ns).or_default().insert(path, json);
    }
    let mut written = 0;
    for (ns, events) in by_ns {
        let out = staging
            .join("assets")
            .join(&ns)
            .join("sounds.json");
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut map = JsonMap::new();
        if out.is_file() {
            let text = fs::read_to_string(&out)?;
            if let Ok(JsonValue::Object(obj)) = serde_json::from_str::<JsonValue>(&text) {
                map = obj;
            }
        }
        for (k, v) in events {
            map.insert(k, v);
        }
        fs::write(&out, serde_json::to_string_pretty(&JsonValue::Object(map))?)?;
        written += 1;
    }
    Ok(written)
}

fn generate_equipments(configs: &LoadedConfigs, staging: &Path) -> Result<()> {
    for (id, value) in &configs.equipments {
        // skip version-branch keys like $$>=1.21.2
        if id.starts_with("$$") {
            // nested map of equipment ids
            if let Some(m) = value.as_mapping() {
                for (ik, iv) in m {
                    if let Some(eid) = ik.as_str() {
                        write_equipment_json(eid, iv, staging)?;
                    }
                }
            }
            continue;
        }
        write_equipment_json(id, value, staging)?;
    }
    Ok(())
}

fn write_equipment_json(id: &str, value: &YamlValue, staging: &Path) -> Result<()> {
    let (ns, path) = split_key(id);
    let out = staging
        .join("assets")
        .join(ns)
        .join("equipment")
        .join(format!("{path}.json"));
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    // Pass through equipment shape loosely: layers from humanoid etc.
    let mut layers = JsonMap::new();
    if let Some(map) = value.as_mapping() {
        for key in ["humanoid", "humanoid_leggings", "wings", "horse_body"] {
            if let Some(v) = map.get(YamlValue::String(key.into())) {
                if let Some(s) = v.as_str() {
                    layers.insert(
                        key.into(),
                        json!([{ "texture": s }]),
                    );
                }
            }
        }
    }
    let doc = json!({ "layers": layers });
    fs::write(out, serde_json::to_string_pretty(&doc)?)?;
    Ok(())
}

fn write_pack_mcmeta_for_target(
    project: &Project,
    target: &ResolvedPackTarget,
    staging: &Path,
) -> Result<()> {
    let base = if project.build.pack.description.is_empty() {
        project.build.project.description.clone()
    } else {
        project.build.pack.description.clone()
    };
    let mut desc = if let Some(custom) = &target.description {
        custom.clone()
    } else {
        strip_minimessage_light(&base)
    };
    if target.name != "default" && target.description.is_none() {
        desc = format!("{desc}\n§8[{}]", target.name);
    }
    let doc = json!({
        "pack": {
            "description": desc,
            "pack_format": target.pack_format,
            "supported_formats": {
                "min_inclusive": target.supported_formats.min_inclusive,
                "max_inclusive": target.supported_formats.max_inclusive
            },
            "min_format": target.min_format,
            "max_format": target.max_format
        }
    });
    fs::write(
        staging.join("pack.mcmeta"),
        serde_json::to_string_pretty(&doc)?,
    )?;
    Ok(())
}

fn strip_minimessage_light(s: &str) -> String {
    // keep it readable in pack.mcmeta; MiniMessage tags are stripped lightly
    let mut out = s.to_string();
    for tag in [
        "<white>",
        "</white>",
        "<gray>",
        "</gray>",
        "<!i>",
        "<bold>",
        "</bold>",
    ] {
        out = out.replace(tag, "");
    }
    out
}

fn yaml_to_json(v: &YamlValue) -> Result<JsonValue> {
    // Stable bridge: YAML -> JSON via serde intermediate string.
    let yaml = serde_yaml::to_string(v)?;
    let json: JsonValue = serde_yaml::from_str(&yaml)
        .map_err(|e| Error::Msg(format!("yaml->json: {e}")))?;
    // serde_yaml can deserialize into serde_json::Value
    Ok(json)
}

fn zip_directory(src: &Path, dst: &Path, zip_cfg: &ZipConfig) -> Result<()> {
    let file = fs::File::create(dst)?;
    let mut zip = ZipWriter::new(file);
    let method = match zip_cfg.method {
        ZipMethod::Deflated => CompressionMethod::Deflated,
        ZipMethod::Stored => CompressionMethod::Stored,
    };
    let mut options = SimpleFileOptions::default()
        .compression_method(method)
        .last_modified_time(
            zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0)
                .unwrap_or_else(|_| zip::DateTime::default_for_write()),
        );
    if matches!(zip_cfg.method, ZipMethod::Deflated) {
        options = options.compression_level(Some(zip_cfg.clamped_level()));
    }

    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let rel = path
            .strip_prefix(src)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(rel, options)?;
        let mut f = fs::File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        zip.write_all(&buf)?;
    }
    zip.finish()?;
    Ok(())
}

