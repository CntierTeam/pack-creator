//! Gradle-inspired `build.pk` DSL — **all pack configuration aggregates here**.
//!
//! `src/main/resourcepack/` holds static assets; optional YAML under
//! `src/main/configuration/` only merges as extras (build.pk wins on key clash).

use crate::config::LoadedConfigs;
use crate::error::{Error, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;

const META_ROOTS: &[&str] = &["project", "mappings", "pack", "export", "zip"];

/// Content section root name in build.pk → canonical LoadedConfigs key.
const CONTENT_ROOTS: &[(&str, &str)] = &[
    ("images", "images"),
    ("image", "images"),
    ("emojis", "emojis"),
    ("emoji", "emojis"),
    ("items", "items"),
    ("item", "items"),
    ("blocks", "blocks"),
    ("block", "blocks"),
    ("entityModels", "entity_models"),
    ("entity_models", "entity_models"),
    ("entities", "entity_models"),
    ("entity", "entity_models"),
    ("lang", "lang"),
    ("languages", "lang"),
    ("language", "lang"),
    ("sounds", "sounds"),
    ("sound", "sounds"),
    ("equipments", "equipments"),
    ("equipment", "equipments"),
    ("furniture", "furniture"),
    ("paintings", "paintings"),
    ("painting", "paintings"),
    ("templates", "templates"),
    ("template", "templates"),
    ("globalVariables", "global-variables"),
    ("global_variables", "global-variables"),
    ("recipes", "recipes"),
    ("recipe", "recipes"),
    ("categories", "categories"),
    ("category", "categories"),
    ("lootTables", "loot-tables"),
    ("loot_tables", "loot-tables"),
    ("gui", "gui"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPk {
    pub project: ProjectMeta,
    pub mappings: MappingsConfig,
    pub pack: PackConfig,
    pub export: ExportConfig,
    #[serde(default)]
    pub zip: ZipConfig,
    /// Aggregated content: canonical section → id → yaml mapping/value.
    #[serde(default)]
    pub contents: BTreeMap<String, BTreeMap<String, YamlValue>>,
}

/// Inclusive pack format range written to `supported_formats` in pack.mcmeta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatRange {
    pub min_inclusive: u32,
    pub max_inclusive: u32,
}

impl FormatRange {
    pub fn single(pf: u32) -> Self {
        Self {
            min_inclusive: pf,
            max_inclusive: pf,
        }
    }
}

impl Default for FormatRange {
    fn default() -> Self {
        FormatRange::single(default_pack_format())
    }
}

/// Per-Minecraft-version zip target (Asteri-style variants).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantExport {
    #[serde(default = "default_pack_format")]
    pub pack_format: u32,
    #[serde(default = "default_min_format")]
    pub min_format: [u32; 2],
    #[serde(default = "default_max_format")]
    pub max_format: [u32; 2],
    #[serde(default = "default_supported_formats")]
    pub supported_formats: FormatRange,
    pub resource_pack_zip: String,
    /// Overlay dirs under `src/main/resourcepack/overlays/<name>/` to merge.
    #[serde(default)]
    pub overlays: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl VariantExport {
    pub fn from_pack_format(pf: u32, zip: impl Into<String>) -> Self {
        Self {
            pack_format: pf,
            min_format: [pf, 0],
            max_format: [pf, 0],
            supported_formats: FormatRange::single(pf),
            resource_pack_zip: zip.into(),
            overlays: Vec::new(),
            description: None,
        }
    }

    pub fn sync_formats_from_pack_format(&mut self) {
        self.min_format = [self.pack_format, 0];
        self.max_format = [self.pack_format, 0];
        self.supported_formats = FormatRange::single(self.pack_format);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum ZipMethod {
    #[default]
    Deflated,
    Stored,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZipConfig {
    #[serde(default)]
    pub method: ZipMethod,
    /// DEFLATE level 0..=9 (ignored for STORED). Default 6 (Asteri fallback).
    #[serde(default = "default_zip_level")]
    pub level: u8,
}

fn default_zip_level() -> u8 {
    6
}

impl Default for ZipConfig {
    fn default() -> Self {
        Self {
            method: ZipMethod::Deflated,
            level: default_zip_level(),
        }
    }
}

impl ZipConfig {
    pub fn clamped_level(&self) -> i64 {
        i64::from(self.level.min(9))
    }
}

/// Resolved zip target used by the build loop.
#[derive(Debug, Clone)]
pub struct ResolvedPackTarget {
    pub name: String,
    pub pack_format: u32,
    pub min_format: [u32; 2],
    pub max_format: [u32; 2],
    pub supported_formats: FormatRange,
    pub resource_pack_zip: String,
    pub overlays: Vec<String>,
    pub description: Option<String>,
}

/// Normalize variant keys: `26.2` → `26_2`.
pub fn normalize_variant_key(name: &str) -> String {
    name.trim().replace('.', "_")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub namespace: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_true")]
    pub enable: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MappingsMode {
    Whole,
    Custom,
}

impl Default for MappingsMode {
    fn default() -> Self {
        Self::Whole
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingsConfig {
    #[serde(default)]
    pub mode: MappingsMode,
    #[serde(default)]
    pub font: FontMappingConfig,
    #[serde(default)]
    pub custom_model_data: CmdMappingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontMappingConfig {
    #[serde(default = "default_codepoint")]
    pub codepoint_starting_value: u32,
    #[serde(default)]
    pub overrides: BTreeMap<String, u32>,
    #[serde(default = "default_true")]
    pub offset_characters: bool,
    #[serde(default = "default_offset_font")]
    pub offset_font: String,
}

fn default_codepoint() -> u32 {
    19_968
}
fn default_offset_font() -> String {
    "minecraft:default".into()
}

impl Default for FontMappingConfig {
    fn default() -> Self {
        let mut overrides = BTreeMap::new();
        overrides.insert("minecraft:default".into(), 57_344);
        Self {
            codepoint_starting_value: default_codepoint(),
            overrides,
            offset_characters: true,
            offset_font: default_offset_font(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdMappingConfig {
    #[serde(default = "default_cmd")]
    pub starting_value: u32,
    #[serde(default)]
    pub overrides: BTreeMap<String, u32>,
}

fn default_cmd() -> u32 {
    10_000
}

impl Default for CmdMappingConfig {
    fn default() -> Self {
        Self {
            starting_value: default_cmd(),
            overrides: BTreeMap::new(),
        }
    }
}

impl Default for MappingsConfig {
    fn default() -> Self {
        Self {
            mode: MappingsMode::Whole,
            font: FontMappingConfig::default(),
            custom_model_data: CmdMappingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackConfig {
    #[serde(default)]
    pub supported_version: SupportedVersion,
    #[serde(default)]
    pub features: FeatureFlags,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_pack_format")]
    pub pack_format: u32,
    #[serde(default = "default_min_format")]
    pub min_format: [u32; 2],
    #[serde(default = "default_max_format")]
    pub max_format: [u32; 2],
    #[serde(default = "default_supported_formats")]
    pub supported_formats: FormatRange,
}

fn default_pack_format() -> u32 {
    34
}
fn default_min_format() -> [u32; 2] {
    [default_pack_format(), 0]
}
fn default_max_format() -> [u32; 2] {
    [default_pack_format(), 0]
}
fn default_supported_formats() -> FormatRange {
    FormatRange::single(default_pack_format())
}

impl PackConfig {
    pub fn sync_formats_from_pack_format(&mut self) {
        self.min_format = [self.pack_format, 0];
        self.max_format = [self.pack_format, 0];
        self.supported_formats = FormatRange::single(self.pack_format);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedVersion {
    #[serde(default = "default_min_ver")]
    pub min: String,
    #[serde(default = "default_max_ver")]
    pub max: String,
}

fn default_min_ver() -> String {
    "1.20.1".into()
}
fn default_max_ver() -> String {
    "1.21.4".into()
}

impl Default for SupportedVersion {
    fn default() -> Self {
        Self {
            min: default_min_ver(),
            max: default_max_ver(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    #[serde(default = "default_true")]
    pub images: bool,
    #[serde(default = "default_true")]
    pub emoji: bool,
    #[serde(default = "default_true")]
    pub lang: bool,
    #[serde(default = "default_true")]
    pub sounds: bool,
    #[serde(default = "default_true")]
    pub equipment: bool,
    #[serde(default = "default_true")]
    pub paintings: bool,
    #[serde(default = "default_true")]
    pub items: bool,
    #[serde(default = "default_true")]
    pub blocks: bool,
    #[serde(default = "default_true")]
    pub furniture: bool,
    #[serde(default = "default_true")]
    pub templates: bool,
    #[serde(default = "default_true")]
    pub placeholders: bool,
    #[serde(default = "default_true")]
    pub recipes: bool,
    #[serde(default = "default_true")]
    pub categories: bool,
    #[serde(default = "default_true")]
    pub loot_tables: bool,
    #[serde(default = "default_true")]
    pub gui: bool,
    #[serde(default = "default_true")]
    pub entities: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            images: true,
            emoji: true,
            lang: true,
            sounds: true,
            equipment: true,
            paintings: true,
            items: true,
            blocks: true,
            furniture: true,
            templates: true,
            placeholders: true,
            recipes: true,
            categories: true,
            loot_tables: true,
            gui: true,
            entities: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    #[serde(default = "default_pack_dir")]
    pub pack_dir: String,
    #[serde(default = "default_zip_out")]
    pub resource_pack_zip: String,
    /// Named Minecraft-version outputs; empty = single zip from `resource_pack_zip`.
    #[serde(default)]
    pub variants: IndexMap<String, VariantExport>,
    /// Built when `build` has no `--variant` filter. Empty = all variant keys.
    #[serde(default)]
    pub default_variants: Vec<String>,
}

fn default_pack_dir() -> String {
    "build/pack".into()
}
fn default_zip_out() -> String {
    "build/resource_pack.zip".into()
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            pack_dir: default_pack_dir(),
            resource_pack_zip: default_zip_out(),
            variants: IndexMap::new(),
            default_variants: Vec::new(),
        }
    }
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            supported_version: SupportedVersion::default(),
            features: FeatureFlags::default(),
            description: String::new(),
            pack_format: default_pack_format(),
            min_format: default_min_format(),
            max_format: default_max_format(),
            supported_formats: default_supported_formats(),
        }
    }
}

impl Default for BuildPk {
    fn default() -> Self {
        Self {
            project: ProjectMeta {
                name: "untitled".into(),
                namespace: "untitled".into(),
                version: "1.0.0".into(),
                author: String::new(),
                description: String::new(),
                enable: true,
            },
            mappings: MappingsConfig::default(),
            pack: PackConfig::default(),
            export: ExportConfig::default(),
            zip: ZipConfig::default(),
            contents: BTreeMap::new(),
        }
    }
}

impl BuildPk {
    pub fn for_new_project(name: &str, namespace: &str) -> Self {
        let mut b = Self::default();
        b.project.name = name.to_string();
        b.project.namespace = namespace.to_string();
        b.pack.description = format!("<white>{name}</white>");
        b.contents = default_scaffold_contents(namespace, name);
        b
    }

    pub fn parse(source: &str) -> Result<Self> {
        parse_build_pk(source)
    }

    /// Resolve which zip targets to build (`filter` empty → defaultVariants / all / single).
    pub fn resolve_pack_targets(&self, filter: &[String]) -> Result<Vec<ResolvedPackTarget>> {
        if self.export.variants.is_empty() {
            if !filter.is_empty() {
                return Err(Error::BuildPk(
                    "build.pk has no export.variants; cannot filter with --variant".into(),
                ));
            }
            return Ok(vec![ResolvedPackTarget {
                name: "default".into(),
                pack_format: self.pack.pack_format,
                min_format: self.pack.min_format,
                max_format: self.pack.max_format,
                supported_formats: self.pack.supported_formats,
                resource_pack_zip: self.export.resource_pack_zip.clone(),
                overlays: Vec::new(),
                description: None,
            }]);
        }

        let wanted: Vec<String> = if filter.is_empty() {
            if self.export.default_variants.is_empty() {
                self.export.variants.keys().cloned().collect()
            } else {
                self.export
                    .default_variants
                    .iter()
                    .map(|s| normalize_variant_key(s))
                    .collect()
            }
        } else {
            filter.iter().map(|s| normalize_variant_key(s)).collect()
        };

        let mut out = Vec::new();
        for key in wanted {
            let Some(v) = self.export.variants.get(&key) else {
                return Err(Error::BuildPk(format!(
                    "unknown export variant `{key}` (known: {})",
                    self.export
                        .variants
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            };
            let overlays = if v.overlays.is_empty() {
                vec![key.clone()]
            } else {
                v.overlays.clone()
            };
            out.push(ResolvedPackTarget {
                name: key,
                pack_format: v.pack_format,
                min_format: v.min_format,
                max_format: v.max_format,
                supported_formats: v.supported_formats,
                resource_pack_zip: v.resource_pack_zip.clone(),
                overlays,
                description: v.description.clone(),
            });
        }
        Ok(out)
    }

    /// Merge aggregated build.pk contents into LoadedConfigs (build.pk wins).
    pub fn apply_to_configs(&self, configs: &mut LoadedConfigs) {
        for (section, entries) in &self.contents {
            match section.as_str() {
                "images" => {
                    for (id, v) in entries {
                        configs.images.insert(id.clone(), v.clone());
                    }
                }
                "emojis" => {
                    for (id, v) in entries {
                        configs.emojis.insert(id.clone(), v.clone());
                    }
                }
                "items" => {
                    for (id, v) in entries {
                        configs.items.insert(id.clone(), v.clone());
                    }
                }
                "entity_models" => {
                    for (id, v) in entries {
                        configs.entity_models.insert(id.clone(), v.clone());
                    }
                }
                "lang" => {
                    for (locale, v) in entries {
                        let slot = configs.langs.entry(locale.clone()).or_default();
                        if let YamlValue::Mapping(m) = v {
                            for (k, val) in m {
                                if let (Some(ks), Some(s)) = (k.as_str(), yaml_scalar_string(val)) {
                                    slot.insert(ks.to_string(), s);
                                }
                            }
                        }
                    }
                }
                "sounds" => {
                    for (id, v) in entries {
                        configs.sounds.insert(id.clone(), v.clone());
                    }
                }
                "equipments" => {
                    for (id, v) in entries {
                        configs.equipments.insert(id.clone(), v.clone());
                    }
                }
                _ => {
                    // keep in raw_sections for export
                    configs
                        .raw_sections
                        .entry(section.clone())
                        .or_default()
                        .push((std::path::PathBuf::from("build.pk"), YamlValue::Mapping({
                            let mut m = serde_yaml::Mapping::new();
                            for (id, v) in entries {
                                m.insert(YamlValue::String(id.clone()), v.clone());
                            }
                            m
                        })));
                }
            }
        }
    }

    pub fn render_dsl(&self) -> String {
        let f = &self.pack.features;
        let mut overrides = String::new();
        for (k, v) in &self.mappings.font.overrides {
            overrides.push_str(&format!("      override(\"{k}\", {v})\n"));
        }
        let mut cmd_ov = String::new();
        for (k, v) in &self.mappings.custom_model_data.overrides {
            cmd_ov.push_str(&format!("      override(\"{k}\", {v})\n"));
        }

        let mut content = String::new();
        for (section, entries) in &self.contents {
            let root = match section.as_str() {
                "images" => "images",
                "emojis" => "emojis",
                "items" => "items",
                "blocks" => "blocks",
                "entity_models" => "entityModels",
                "lang" => "lang",
                "sounds" => "sounds",
                "equipments" => "equipments",
                "furniture" => "furniture",
                "paintings" => "paintings",
                "templates" => "templates",
                "global-variables" => "globalVariables",
                "recipes" => "recipes",
                "categories" => "categories",
                "loot-tables" => "lootTables",
                "gui" => "gui",
                other => other,
            };
            content.push_str(&format!("\n{root} {{\n"));
            for (id, val) in entries {
                match val {
                    YamlValue::Mapping(_) => {
                        content.push_str(&format!(
                            "  \"{}\" {}\n",
                            escape(id),
                            render_yaml_as_dsl(val, 2)
                        ));
                    }
                    _ => {
                        content.push_str(&format!(
                            "  \"{}\" = {}\n",
                            escape(id),
                            render_yaml_scalar(val)
                        ));
                    }
                }
            }
            content.push_str("}\n");
        }

        format!(
            r#"// PackCreator build.pk — all configuration aggregates in this file.
// Static assets live under src/main/resourcepack/

project {{
  name = "{name}"
  namespace = "{ns}"
  version = "{ver}"
  author = "{author}"
  description = "{desc}"
  enable = {enable}
}}

mappings {{
  mode = {mode}
  font {{
    codepointStartingValue = {cp}
    offsetCharacters = {off}
    offsetFont = "{off_font}"
{overrides}  }}
  customModelData {{
    startingValue = {cmd}
{cmd_ov}  }}
}}

pack {{
  supportedVersion {{
    min = "{min}"
    max = "{max}"
  }}
  packFormat = {pf}
  minFormat = [{min_f0}, {min_f1}]
  maxFormat = [{max_f0}, {max_f1}]
  supportedFormats {{
    minInclusive = {sf_min}
    maxInclusive = {sf_max}
  }}
  description = "{pdesc}"
  features {{
    images = {images}
    emoji = {emoji}
    lang = {lang}
    sounds = {sounds}
    equipment = {equipment}
    paintings = {paintings}
    items = {items}
    blocks = {blocks}
    furniture = {furniture}
    templates = {templates}
    placeholders = {placeholders}
    recipes = {recipes}
    categories = {categories}
    lootTables = {loot}
    gui = {gui}
    entities = {entities}
  }}
}}

export {{
  packDir = "{pack_dir}"
  resourcePackZip = "{zip}"
{variants_block}{default_variants_block}}}

zip {{
  method = {zip_method}
  level = {zip_level}
}}
{content}"#,
            name = escape(&self.project.name),
            ns = escape(&self.project.namespace),
            ver = escape(&self.project.version),
            author = escape(&self.project.author),
            desc = escape(&self.project.description),
            enable = self.project.enable,
            mode = match self.mappings.mode {
                MappingsMode::Whole => "WHOLE",
                MappingsMode::Custom => "CUSTOM",
            },
            cp = self.mappings.font.codepoint_starting_value,
            off = self.mappings.font.offset_characters,
            off_font = escape(&self.mappings.font.offset_font),
            overrides = overrides,
            cmd = self.mappings.custom_model_data.starting_value,
            cmd_ov = cmd_ov,
            min = escape(&self.pack.supported_version.min),
            max = escape(&self.pack.supported_version.max),
            pf = self.pack.pack_format,
            min_f0 = self.pack.min_format[0],
            min_f1 = self.pack.min_format[1],
            max_f0 = self.pack.max_format[0],
            max_f1 = self.pack.max_format[1],
            sf_min = self.pack.supported_formats.min_inclusive,
            sf_max = self.pack.supported_formats.max_inclusive,
            pdesc = escape(&self.pack.description),
            images = f.images,
            emoji = f.emoji,
            lang = f.lang,
            sounds = f.sounds,
            equipment = f.equipment,
            paintings = f.paintings,
            items = f.items,
            blocks = f.blocks,
            furniture = f.furniture,
            templates = f.templates,
            placeholders = f.placeholders,
            recipes = f.recipes,
            categories = f.categories,
            loot = f.loot_tables,
            gui = f.gui,
            entities = f.entities,
            pack_dir = escape(&self.export.pack_dir),
            zip = escape(&self.export.resource_pack_zip),
            variants_block = render_variants_block(&self.export),
            default_variants_block = render_default_variants(&self.export),
            zip_method = match self.zip.method {
                ZipMethod::Deflated => "DEFLATED",
                ZipMethod::Stored => "STORED",
            },
            zip_level = self.zip.level.min(9),
            content = content,
        )
    }
}

fn render_variants_block(export: &ExportConfig) -> String {
    if export.variants.is_empty() {
        return String::new();
    }
    let mut out = String::from("  variants {\n");
    for (name, v) in &export.variants {
        out.push_str(&format!("    \"{}\" {{\n", escape(name)));
        out.push_str(&format!("      packFormat = {}\n", v.pack_format));
        out.push_str(&format!(
            "      minFormat = [{}, {}]\n",
            v.min_format[0], v.min_format[1]
        ));
        out.push_str(&format!(
            "      maxFormat = [{}, {}]\n",
            v.max_format[0], v.max_format[1]
        ));
        out.push_str("      supportedFormats {\n");
        out.push_str(&format!(
            "        minInclusive = {}\n",
            v.supported_formats.min_inclusive
        ));
        out.push_str(&format!(
            "        maxInclusive = {}\n",
            v.supported_formats.max_inclusive
        ));
        out.push_str("      }\n");
        out.push_str(&format!(
            "      resourcePackZip = \"{}\"\n",
            escape(&v.resource_pack_zip)
        ));
        if !v.overlays.is_empty() {
            let parts: Vec<String> = v
                .overlays
                .iter()
                .map(|o| format!("\"{}\"", escape(o)))
                .collect();
            out.push_str(&format!("      overlays = [{}]\n", parts.join(", ")));
        }
        if let Some(desc) = &v.description {
            out.push_str(&format!("      description = \"{}\"\n", escape(desc)));
        }
        out.push_str("    }\n");
    }
    out.push_str("  }\n");
    out
}

fn render_default_variants(export: &ExportConfig) -> String {
    if export.default_variants.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = export
        .default_variants
        .iter()
        .map(|v| format!("\"{}\"", escape(v)))
        .collect();
    format!("  defaultVariants = [{}]\n", parts.join(", "))
}

fn yaml_scalar_string(v: &YamlValue) -> Option<String> {
    match v {
        YamlValue::String(s) => Some(s.clone()),
        YamlValue::Number(n) => Some(n.to_string()),
        YamlValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn default_scaffold_contents(ns: &str, name: &str) -> BTreeMap<String, BTreeMap<String, YamlValue>> {
    let mut contents = BTreeMap::new();

    let mut images = BTreeMap::new();
    images.insert(
        format!("{ns}:example_icon"),
        yaml_map(&[
            ("height", YamlValue::Number(16.into())),
            ("ascent", YamlValue::Number(12.into())),
            ("font", YamlValue::String("minecraft:default".into())),
            ("file", YamlValue::String(format!("{ns}:font/example_icon.png"))),
            ("grid_size", YamlValue::String("1,1".into())),
        ]),
    );
    images.insert(
        format!("{ns}:main_gui"),
        yaml_map(&[
            ("height", YamlValue::Number(140.into())),
            ("ascent", YamlValue::Number(18.into())),
            ("font", YamlValue::String("minecraft:gui".into())),
            ("file", YamlValue::String(format!("{ns}:font/gui/main_gui.png"))),
        ]),
    );
    contents.insert("images".into(), images);

    let mut emojis = BTreeMap::new();
    emojis.insert(
        format!("{ns}:smile"),
        yaml_map(&[
            ("image", YamlValue::String(format!("{ns}:example_icon:0:0"))),
            (
                "keywords",
                YamlValue::Sequence(vec![
                    YamlValue::String(":)".into()),
                    YamlValue::String(":smile:".into()),
                ]),
            ),
            (
                "content",
                YamlValue::String(format!(
                    "<white><image:{ns}:example_icon:0:0></white>"
                )),
            ),
        ]),
    );
    contents.insert("emojis".into(), emojis);

    let mut items = BTreeMap::new();
    items.insert(
        format!("{ns}:demo_item"),
        yaml_map(&[
            ("material", YamlValue::String("PAPER".into())),
            (
                "data",
                yaml_map(&[(
                    "display-name",
                    YamlValue::String("<!i><white>Demo Item</white>".into()),
                )]),
            ),
            (
                "model",
                yaml_map(&[
                    ("type", YamlValue::String("minecraft:model".into())),
                    ("path", YamlValue::String(format!("{ns}:item/demo_item"))),
                    (
                        "generation",
                        yaml_map(&[
                            (
                                "parent",
                                YamlValue::String("minecraft:item/generated".into()),
                            ),
                            (
                                "textures",
                                yaml_map(&[(
                                    "layer0",
                                    YamlValue::String(format!("{ns}:item/demo_item")),
                                )]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]),
    );
    items.insert(
        format!("{ns}:gui_next"),
        yaml_map(&[
            ("material", YamlValue::String("PAPER".into())),
            (
                "model",
                yaml_map(&[
                    ("type", YamlValue::String("minecraft:model".into())),
                    ("path", YamlValue::String(format!("{ns}:item/gui/next"))),
                    (
                        "generation",
                        yaml_map(&[
                            (
                                "parent",
                                YamlValue::String("minecraft:item/generated".into()),
                            ),
                            (
                                "textures",
                                yaml_map(&[(
                                    "layer0",
                                    YamlValue::String(format!("{ns}:item/gui/next")),
                                )]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]),
    );
    contents.insert("items".into(), items);

    let mut entities = BTreeMap::new();
    entities.insert(
        format!("{ns}:demo_cow"),
        yaml_map(&[
            (
                "model",
                yaml_map(&[
                    ("path", YamlValue::String(format!("{ns}:entity/demo_cow"))),
                    ("parent", YamlValue::String("minecraft:block/block".into())),
                    (
                        "textures",
                        yaml_map(&[("all", YamlValue::String(format!("{ns}:entity/demo_cow")))]),
                    ),
                ]),
            ),
            (
                "replace_textures",
                YamlValue::Sequence(vec![yaml_map(&[
                    ("from", YamlValue::String(format!("{ns}:entity/demo_cow"))),
                    (
                        "to",
                        YamlValue::String("minecraft:entity/cow/cow.png".into()),
                    ),
                ])]),
            ),
        ]),
    );
    contents.insert("entity_models".into(), entities);

    let mut lang = BTreeMap::new();
    let pack_name_key = format!("pack.{ns}.name");
    lang.insert(
        "en_us".into(),
        yaml_map_owned(&[(pack_name_key.clone(), YamlValue::String(name.into()))]),
    );
    lang.insert(
        "zh_cn".into(),
        yaml_map_owned(&[(pack_name_key, YamlValue::String(name.into()))]),
    );
    contents.insert("lang".into(), lang);

    let mut sounds = BTreeMap::new();
    sounds.insert(
        format!("{ns}:demo_click"),
        yaml_map(&[
            ("replace", YamlValue::Bool(false)),
            (
                "subtitle",
                YamlValue::String(format!("subtitles.{ns}.demo_click")),
            ),
            (
                "sounds",
                YamlValue::Sequence(vec![YamlValue::String(format!("{ns}:ui/click"))]),
            ),
        ]),
    );
    contents.insert("sounds".into(), sounds);

    let mut equipments = BTreeMap::new();
    equipments.insert(
        format!("{ns}:demo_trim"),
        yaml_map(&[
            ("type", YamlValue::String("component".into())),
            (
                "humanoid",
                YamlValue::String(format!("{ns}:entity/equipment/humanoid/demo")),
            ),
            (
                "humanoid_leggings",
                YamlValue::String(format!("{ns}:entity/equipment/humanoid_leggings/demo")),
            ),
        ]),
    );
    contents.insert("equipments".into(), equipments);

    let mut blocks = BTreeMap::new();
    blocks.insert(
        format!("{ns}:demo_block"),
        yaml_map(&[
            (
                "settings",
                yaml_map(&[
                    ("hardness", YamlValue::Number(serde_yaml::Number::from(1.5))),
                    (
                        "resistance",
                        YamlValue::Number(serde_yaml::Number::from(1.5)),
                    ),
                ]),
            ),
            (
                "state",
                yaml_map(&[
                    ("auto_state", YamlValue::String("solid".into())),
                    (
                        "model",
                        yaml_map(&[
                            ("type", YamlValue::String("minecraft:model".into())),
                            (
                                "path",
                                YamlValue::String(format!("{ns}:block/demo_block")),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]),
    );
    contents.insert("blocks".into(), blocks);

    let mut furniture = BTreeMap::new();
    furniture.insert(
        format!("{ns}:demo_chair"),
        yaml_map(&[(
            "settings",
            yaml_map(&[("item", YamlValue::String(format!("{ns}:demo_item")))]),
        )]),
    );
    contents.insert("furniture".into(), furniture);

    let mut paintings = BTreeMap::new();
    paintings.insert(
        format!("{ns}:demo_art"),
        yaml_map(&[
            ("width", YamlValue::Number(1.into())),
            ("height", YamlValue::Number(1.into())),
            ("asset_id", YamlValue::String(format!("{ns}:demo_art"))),
            ("title", YamlValue::String("<white>Demo Art</white>".into())),
            (
                "author",
                YamlValue::String("<gray>PackCreator</gray>".into()),
            ),
        ]),
    );
    contents.insert("paintings".into(), paintings);

    let mut templates = BTreeMap::new();
    templates.insert(
        format!("{ns}:placeholder/basic"),
        yaml_map(&[(
            "content",
            YamlValue::String("<gray>${text:-hello}</gray>".into()),
        )]),
    );
    contents.insert("templates".into(), templates);

    let mut globals = BTreeMap::new();
    globals.insert(
        format!("{ns}:welcome"),
        YamlValue::String(format!("<white>Welcome to {name}</white>")),
    );
    contents.insert("global-variables".into(), globals);

    let mut recipes = BTreeMap::new();
    recipes.insert(
        format!("{ns}:demo_craft"),
        yaml_map(&[
            ("type", YamlValue::String("shaped".into())),
            (
                "pattern",
                YamlValue::Sequence(vec![
                    YamlValue::String(" A ".into()),
                    YamlValue::String(" A ".into()),
                    YamlValue::String(" B ".into()),
                ]),
            ),
            (
                "ingredients",
                yaml_map(&[
                    ("A", YamlValue::String("PAPER".into())),
                    ("B", YamlValue::String("STICK".into())),
                ]),
            ),
            (
                "result",
                yaml_map(&[
                    ("id", YamlValue::String(format!("{ns}:demo_item"))),
                    ("count", YamlValue::Number(1.into())),
                ]),
            ),
        ]),
    );
    contents.insert("recipes".into(), recipes);

    let mut categories = BTreeMap::new();
    categories.insert(
        format!("{ns}:demo"),
        yaml_map(&[
            ("name", YamlValue::String("<white>Demo</white>".into())),
            ("icon", YamlValue::String(format!("{ns}:demo_item"))),
            (
                "list",
                YamlValue::Sequence(vec![YamlValue::String(format!("{ns}:demo_item"))]),
            ),
        ]),
    );
    contents.insert("categories".into(), categories);

    let mut loot = BTreeMap::new();
    loot.insert(
        format!("{ns}:demo_drop"),
        yaml_map(&[(
            "pools",
            YamlValue::Sequence(vec![yaml_map(&[
                ("rolls", YamlValue::Number(1.into())),
                (
                    "entries",
                    YamlValue::Sequence(vec![yaml_map(&[
                        ("type", YamlValue::String("item".into())),
                        ("item", YamlValue::String(format!("{ns}:demo_item"))),
                        ("weight", YamlValue::Number(1.into())),
                    ])]),
                ),
            ])]),
        )]),
    );
    contents.insert("loot-tables".into(), loot);

    contents
}

fn yaml_map(entries: &[(&str, YamlValue)]) -> YamlValue {
    let mut m = serde_yaml::Mapping::new();
    for (k, v) in entries {
        m.insert(YamlValue::String((*k).to_string()), v.clone());
    }
    YamlValue::Mapping(m)
}

fn yaml_map_owned(entries: &[(String, YamlValue)]) -> YamlValue {
    let mut m = serde_yaml::Mapping::new();
    for (k, v) in entries {
        m.insert(YamlValue::String(k.clone()), v.clone());
    }
    YamlValue::Mapping(m)
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_yaml_as_dsl(v: &YamlValue, indent: usize) -> String {
    let pad = " ".repeat(indent);
    match v {
        YamlValue::Mapping(m) => {
            let mut out = String::from("{\n");
            for (k, val) in m {
                let key = k.as_str().unwrap_or("?");
                let key_render = if key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    key.to_string()
                } else {
                    format!("\"{}\"", escape(key))
                };
                match val {
                    YamlValue::Mapping(_) => {
                        out.push_str(&format!(
                            "{pad}  {key_render} {}\n",
                            render_yaml_as_dsl(val, indent + 2)
                        ));
                    }
                    _ => {
                        out.push_str(&format!(
                            "{pad}  {key_render} = {}\n",
                            render_yaml_scalar(val)
                        ));
                    }
                }
            }
            out.push_str(&format!("{pad}}}"));
            out
        }
        YamlValue::Sequence(seq) => {
            let parts: Vec<String> = seq.iter().map(render_yaml_scalar).collect();
            format!("[{}]", parts.join(", "))
        }
        other => render_yaml_scalar(other),
    }
}

fn render_yaml_scalar(v: &YamlValue) -> String {
    match v {
        YamlValue::Bool(b) => b.to_string(),
        YamlValue::Number(n) => n.to_string(),
        YamlValue::String(s) => format!("\"{}\"", escape(s)),
        YamlValue::Null => "null".into(),
        YamlValue::Sequence(seq) => {
            let parts: Vec<String> = seq.iter().map(render_yaml_scalar).collect();
            format!("[{}]", parts.join(", "))
        }
        YamlValue::Mapping(_) => render_yaml_as_dsl(v, 0),
        YamlValue::Tagged(t) => render_yaml_scalar(&t.value),
    }
}

#[derive(Debug, Clone)]
enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Ident(String),
    Block(IndexMap<String, Value>),
    List(Vec<Value>),
    Call { name: String, args: Vec<Value> },
}

struct Parser<'a> {
    src: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self { src, i: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.i..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.i += ch.len_utf8();
        Some(ch)
    }

    fn skip_ws_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.bump();
            }
            if self.src[self.i..].starts_with("//") {
                while let Some(c) = self.bump() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            break;
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        self.skip_ws_comments();
        let start = self.i;
        let Some(first) = self.peek() else {
            return Err(Error::BuildPk("unexpected eof".into()));
        };
        if !(first.is_ascii_alphabetic() || first == '_') {
            return Err(Error::BuildPk(format!(
                "expected ident at {}: found {first:?}",
                self.i
            )));
        }
        self.bump();
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            self.bump();
        }
        Ok(self.src[start..self.i].to_string())
    }

    fn expect_char(&mut self, want: char) -> Result<()> {
        self.skip_ws_comments();
        match self.bump() {
            Some(c) if c == want => Ok(()),
            other => Err(Error::BuildPk(format!(
                "expected {want:?} at {}, got {other:?}",
                self.i
            ))),
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        self.skip_ws_comments();
        self.expect_char('"')?;
        let mut out = String::new();
        while let Some(c) = self.bump() {
            match c {
                '"' => return Ok(out),
                '\\' => match self.bump() {
                    Some(n) => out.push(n),
                    None => return Err(Error::BuildPk("bad escape".into())),
                },
                other => out.push(other),
            }
        }
        Err(Error::BuildPk("unterminated string".into()))
    }

    fn parse_key(&mut self) -> Result<String> {
        self.skip_ws_comments();
        if self.peek() == Some('"') {
            self.parse_string()
        } else {
            self.expect_ident()
        }
    }

    fn parse_list(&mut self) -> Result<Value> {
        self.expect_char('[')?;
        let mut items = Vec::new();
        loop {
            self.skip_ws_comments();
            if self.peek() == Some(']') {
                self.bump();
                break;
            }
            items.push(self.parse_value()?);
            self.skip_ws_comments();
            if self.peek() == Some(',') {
                self.bump();
                continue;
            }
            if self.peek() == Some(']') {
                self.bump();
                break;
            }
            return Err(Error::BuildPk("expected , or ] in list".into()));
        }
        Ok(Value::List(items))
    }

    fn parse_value(&mut self) -> Result<Value> {
        self.skip_ws_comments();
        match self.peek() {
            Some('"') => Ok(Value::String(self.parse_string()?)),
            Some('{') => Ok(Value::Block(self.parse_block_body()?)),
            Some('[') => self.parse_list(),
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let start = self.i;
                if c == '-' {
                    self.bump();
                }
                while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                    self.bump();
                }
                if self.peek() == Some('.') {
                    self.bump();
                    while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                        self.bump();
                    }
                    let n: f64 = self.src[start..self.i]
                        .parse()
                        .map_err(|e| Error::BuildPk(format!("bad float: {e}")))?;
                    Ok(Value::Float(n))
                } else {
                    let n: i64 = self.src[start..self.i]
                        .parse()
                        .map_err(|e| Error::BuildPk(format!("bad int: {e}")))?;
                    Ok(Value::Int(n))
                }
            }
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                let ident = self.expect_ident()?;
                self.skip_ws_comments();
                if self.peek() == Some('(') {
                    self.bump();
                    let mut args = Vec::new();
                    self.skip_ws_comments();
                    if self.peek() != Some(')') {
                        loop {
                            args.push(self.parse_value()?);
                            self.skip_ws_comments();
                            if self.peek() == Some(',') {
                                self.bump();
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect_char(')')?;
                    Ok(Value::Call { name: ident, args })
                } else if ident == "true" {
                    Ok(Value::Bool(true))
                } else if ident == "false" {
                    Ok(Value::Bool(false))
                } else if ident == "null" {
                    Ok(Value::Ident("null".into()))
                } else {
                    Ok(Value::Ident(ident))
                }
            }
            other => Err(Error::BuildPk(format!(
                "unexpected value at {}: {other:?}",
                self.i
            ))),
        }
    }

    fn parse_block_body(&mut self) -> Result<IndexMap<String, Value>> {
        self.expect_char('{')?;
        let mut map = IndexMap::new();
        loop {
            self.skip_ws_comments();
            if self.peek() == Some('}') {
                self.bump();
                break;
            }
            let key = self.parse_key()?;
            self.skip_ws_comments();
            if self.peek() == Some('{') {
                map.insert(key, Value::Block(self.parse_block_body()?));
                continue;
            }
            if self.peek() == Some('(') {
                self.bump();
                let mut args = Vec::new();
                self.skip_ws_comments();
                if self.peek() != Some(')') {
                    loop {
                        args.push(self.parse_value()?);
                        self.skip_ws_comments();
                        if self.peek() == Some(',') {
                            self.bump();
                            continue;
                        }
                        break;
                    }
                }
                self.expect_char(')')?;
                let entry_key = format!("__call_{}_{}", map.len(), key);
                map.insert(entry_key, Value::Call { name: key, args });
                continue;
            }
            self.expect_char('=')?;
            let val = self.parse_value()?;
            map.insert(key, val);
        }
        Ok(map)
    }

    fn parse_file(&mut self) -> Result<IndexMap<String, Value>> {
        let mut root = IndexMap::new();
        loop {
            self.skip_ws_comments();
            if self.i >= self.src.len() {
                break;
            }
            let name = self.expect_ident()?;
            let body = Value::Block(self.parse_block_body()?);
            root.insert(name, body);
        }
        Ok(root)
    }
}

fn as_str(v: &Value) -> Result<String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Ident(s) => Ok(s.clone()),
        Value::Int(i) => Ok(i.to_string()),
        Value::Float(f) => Ok(f.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(Error::BuildPk("expected string".into())),
    }
}

fn as_bool(v: &Value) -> Result<bool> {
    match v {
        Value::Bool(b) => Ok(*b),
        Value::Ident(s) if s == "true" => Ok(true),
        Value::Ident(s) if s == "false" => Ok(false),
        _ => Err(Error::BuildPk("expected bool".into())),
    }
}

fn as_u32(v: &Value) -> Result<u32> {
    match v {
        Value::Int(i) if *i >= 0 => Ok(*i as u32),
        _ => Err(Error::BuildPk("expected u32".into())),
    }
}

fn as_u32_pair(v: &Value) -> Result<[u32; 2]> {
    match v {
        Value::List(items) if items.len() == 2 => Ok([as_u32(&items[0])?, as_u32(&items[1])?]),
        Value::Int(i) if *i >= 0 => Ok([*i as u32, 0]),
        _ => Err(Error::BuildPk("expected [major, minor] format pair".into())),
    }
}

fn as_string_list(v: &Value) -> Result<Vec<String>> {
    match v {
        Value::List(items) => items.iter().map(as_str).collect(),
        other => Ok(vec![as_str(other)?]),
    }
}

fn parse_format_fields(
    m: &IndexMap<String, Value>,
    pack_format: &mut u32,
    min_format: &mut [u32; 2],
    max_format: &mut [u32; 2],
    supported: &mut FormatRange,
) -> Result<()> {
    let mut pf_set = false;
    let mut min_set = false;
    let mut max_set = false;
    let mut sf_min_set = false;
    let mut sf_max_set = false;
    if let Some(v) = m.get("packFormat") {
        *pack_format = as_u32(v)?;
        pf_set = true;
    }
    if let Some(v) = m.get("minFormat") {
        *min_format = as_u32_pair(v)?;
        min_set = true;
    }
    if let Some(v) = m.get("maxFormat") {
        *max_format = as_u32_pair(v)?;
        max_set = true;
    }
    if let Some(sf) = m.get("supportedFormats") {
        let s = block(sf)?;
        if let Some(v) = s.get("minInclusive") {
            supported.min_inclusive = as_u32(v)?;
            sf_min_set = true;
        }
        if let Some(v) = s.get("maxInclusive") {
            supported.max_inclusive = as_u32(v)?;
            sf_max_set = true;
        }
    }
    if pf_set {
        if !min_set {
            *min_format = [*pack_format, 0];
        }
        if !max_set {
            *max_format = [*pack_format, 0];
        }
        if !sf_min_set {
            supported.min_inclusive = *pack_format;
        }
        if !sf_max_set {
            supported.max_inclusive = *pack_format;
        }
    }
    Ok(())
}

fn block<'a>(v: &'a Value) -> Result<&'a IndexMap<String, Value>> {
    match v {
        Value::Block(m) => Ok(m),
        _ => Err(Error::BuildPk("expected block".into())),
    }
}

fn value_to_yaml(v: &Value) -> YamlValue {
    match v {
        Value::String(s) => YamlValue::String(s.clone()),
        Value::Ident(s) if s == "null" => YamlValue::Null,
        Value::Ident(s) => YamlValue::String(s.clone()),
        Value::Int(i) => YamlValue::Number((*i).into()),
        Value::Float(f) => YamlValue::Number(serde_yaml::Number::from(*f)),
        Value::Bool(b) => YamlValue::Bool(*b),
        Value::List(items) => YamlValue::Sequence(items.iter().map(value_to_yaml).collect()),
        Value::Block(m) => {
            let mut out = serde_yaml::Mapping::new();
            for (k, val) in m {
                if k.starts_with("__call_") {
                    continue;
                }
                out.insert(YamlValue::String(k.clone()), value_to_yaml(val));
            }
            YamlValue::Mapping(out)
        }
        Value::Call { name, args } => {
            // represent as mapping for odd cases
            let mut m = serde_yaml::Mapping::new();
            m.insert(
                YamlValue::String("call".into()),
                YamlValue::String(name.clone()),
            );
            m.insert(
                YamlValue::String("args".into()),
                YamlValue::Sequence(args.iter().map(value_to_yaml).collect()),
            );
            YamlValue::Mapping(m)
        }
    }
}

fn ingest_content_block(
    contents: &mut BTreeMap<String, BTreeMap<String, YamlValue>>,
    canon: &str,
    body: &IndexMap<String, Value>,
) {
    if canon == "gui" {
        // gui { images {..} items {..} } OR gui { "id" { font/file... } }
        if let Some(Value::Block(images)) = body.get("images") {
            ingest_content_block(contents, "images", images);
        }
        if let Some(Value::Block(items)) = body.get("items") {
            ingest_content_block(contents, "items", items);
        }
        for (k, v) in body {
            if k == "images" || k == "items" || k.starts_with("__call_") {
                continue;
            }
            // bare gui entries → images
            let slot = contents.entry("images".into()).or_default();
            slot.insert(k.clone(), value_to_yaml(v));
        }
        return;
    }

    let slot = contents.entry(canon.to_string()).or_default();
    for (k, v) in body {
        if k.starts_with("__call_") {
            continue;
        }
        slot.insert(k.clone(), value_to_yaml(v));
    }
}

fn parse_build_pk(source: &str) -> Result<BuildPk> {
    let mut p = Parser::new(source);
    let root = p.parse_file()?;
    let mut out = BuildPk::default();
    out.project.name.clear();
    out.project.namespace.clear();

    if let Some(proj) = root.get("project") {
        let m = block(proj)?;
        if let Some(v) = m.get("name") {
            out.project.name = as_str(v)?;
        }
        if let Some(v) = m.get("namespace") {
            out.project.namespace = as_str(v)?;
        }
        if let Some(v) = m.get("version") {
            out.project.version = as_str(v)?;
        }
        if let Some(v) = m.get("author") {
            out.project.author = as_str(v)?;
        }
        if let Some(v) = m.get("description") {
            out.project.description = as_str(v)?;
        }
        if let Some(v) = m.get("enable") {
            out.project.enable = as_bool(v)?;
        }
    }

    if let Some(map) = root.get("mappings") {
        let m = block(map)?;
        if let Some(v) = m.get("mode") {
            let s = as_str(v)?;
            out.mappings.mode = match s.to_ascii_uppercase().as_str() {
                "WHOLE" => MappingsMode::Whole,
                "CUSTOM" => MappingsMode::Custom,
                other => {
                    return Err(Error::BuildPk(format!("unknown mappings mode: {other}")))
                }
            };
        }
        if let Some(font) = m.get("font") {
            let f = block(font)?;
            if let Some(v) = f.get("codepointStartingValue") {
                out.mappings.font.codepoint_starting_value = as_u32(v)?;
            }
            if let Some(v) = f.get("offsetCharacters") {
                out.mappings.font.offset_characters = as_bool(v)?;
            }
            if let Some(v) = f.get("offsetFont") {
                out.mappings.font.offset_font = as_str(v)?;
            }
            for (_k, v) in f {
                if let Value::Call { name, args } = v {
                    if name == "override" && args.len() == 2 {
                        out.mappings
                            .font
                            .overrides
                            .insert(as_str(&args[0])?, as_u32(&args[1])?);
                    }
                }
            }
        }
        if let Some(cmd) = m.get("customModelData") {
            let c = block(cmd)?;
            if let Some(v) = c.get("startingValue") {
                out.mappings.custom_model_data.starting_value = as_u32(v)?;
            }
            for (_k, v) in c {
                if let Value::Call { name, args } = v {
                    if name == "override" && args.len() == 2 {
                        out.mappings
                            .custom_model_data
                            .overrides
                            .insert(as_str(&args[0])?, as_u32(&args[1])?);
                    }
                }
            }
        }
    }

    if let Some(pack) = root.get("pack") {
        let m = block(pack)?;
        parse_format_fields(
            m,
            &mut out.pack.pack_format,
            &mut out.pack.min_format,
            &mut out.pack.max_format,
            &mut out.pack.supported_formats,
        )?;
        if let Some(v) = m.get("description") {
            out.pack.description = as_str(v)?;
        }
        if let Some(sv) = m.get("supportedVersion") {
            let s = block(sv)?;
            if let Some(v) = s.get("min") {
                out.pack.supported_version.min = as_str(v)?;
            }
            if let Some(v) = s.get("max") {
                out.pack.supported_version.max = as_str(v)?;
            }
        }
        if let Some(feat) = m.get("features") {
            let f = block(feat)?;
            let flags = &mut out.pack.features;
            macro_rules! flag {
                ($name:ident, $key:expr) => {
                    if let Some(v) = f.get($key) {
                        flags.$name = as_bool(v)?;
                    }
                };
            }
            flag!(images, "images");
            flag!(emoji, "emoji");
            flag!(lang, "lang");
            flag!(sounds, "sounds");
            flag!(equipment, "equipment");
            flag!(paintings, "paintings");
            flag!(items, "items");
            flag!(blocks, "blocks");
            flag!(furniture, "furniture");
            flag!(templates, "templates");
            flag!(placeholders, "placeholders");
            flag!(recipes, "recipes");
            flag!(categories, "categories");
            flag!(loot_tables, "lootTables");
            flag!(gui, "gui");
            flag!(entities, "entities");
        }
    }

    if let Some(exp) = root.get("export") {
        let m = block(exp)?;
        if let Some(v) = m.get("packDir").or_else(|| m.get("pack_dir")) {
            out.export.pack_dir = as_str(v)?;
        }
        if let Some(v) = m.get("resourcePackZip") {
            out.export.resource_pack_zip = as_str(v)?;
        }
        if let Some(v) = m.get("defaultVariants") {
            out.export.default_variants = as_string_list(v)?
                .into_iter()
                .map(|s| normalize_variant_key(&s))
                .collect();
        }
        if let Some(variants) = m.get("variants") {
            let vb = block(variants)?;
            for (raw_key, val) in vb {
                if raw_key.starts_with("__call_") {
                    continue;
                }
                let key = normalize_variant_key(raw_key);
                let body = block(val)?;
                let mut ve = VariantExport::from_pack_format(
                    out.pack.pack_format,
                    format!("build/resource_pack_{key}.zip"),
                );
                parse_format_fields(
                    body,
                    &mut ve.pack_format,
                    &mut ve.min_format,
                    &mut ve.max_format,
                    &mut ve.supported_formats,
                )?;
                if let Some(v) = body.get("resourcePackZip") {
                    ve.resource_pack_zip = as_str(v)?;
                }
                if let Some(v) = body.get("overlays") {
                    ve.overlays = as_string_list(v)?;
                }
                if let Some(v) = body.get("description") {
                    ve.description = Some(as_str(v)?);
                }
                out.export.variants.insert(key, ve);
            }
        }
    }

    if let Some(zip) = root.get("zip") {
        let m = block(zip)?;
        if let Some(v) = m.get("method") {
            let s = as_str(v)?;
            out.zip.method = match s.to_ascii_uppercase().as_str() {
                "DEFLATED" | "DEFLATE" => ZipMethod::Deflated,
                "STORED" | "STORE" | "NONE" => ZipMethod::Stored,
                other => {
                    return Err(Error::BuildPk(format!("unknown zip.method: {other}")))
                }
            };
        }
        if let Some(v) = m.get("level") {
            let level = as_u32(v)?;
            if level > 9 {
                return Err(Error::BuildPk("zip.level must be 0..=9".into()));
            }
            out.zip.level = level as u8;
        }
    }

    // Aggregate all content roots from build.pk
    for (name, val) in &root {
        if META_ROOTS.contains(&name.as_str()) {
            continue;
        }
        let Some((_, canon)) = CONTENT_ROOTS.iter().find(|(a, _)| *a == name.as_str()) else {
            // unknown root still ingested under its name
            if let Value::Block(body) = val {
                ingest_content_block(&mut out.contents, name, body);
            }
            continue;
        };
        if let Value::Block(body) = val {
            ingest_content_block(&mut out.contents, canon, body);
        }
    }

    if out.project.name.is_empty() || out.project.namespace.is_empty() {
        return Err(Error::BuildPk(
            "project.name and project.namespace are required".into(),
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_default() {
        let b = BuildPk::for_new_project("demo", "demo");
        let text = b.render_dsl();
        let parsed = BuildPk::parse(&text).unwrap();
        assert_eq!(parsed.project.name, "demo");
        assert_eq!(parsed.mappings.mode, MappingsMode::Whole);
        assert!(parsed.contents.contains_key("items"));
        assert!(parsed.contents["items"].contains_key("demo:demo_item"));
        assert!(parsed.contents.contains_key("images"));
        assert!(parsed.contents["images"].contains_key("demo:main_gui"));
        assert!(parsed.contents.contains_key("entity_models"));
    }

    #[test]
    fn content_string_keys_and_lists() {
        let src = r#"
project { name = "p" namespace = "ns" }
items {
  "ns:sword" {
    material = "DIAMOND_SWORD"
    model {
      path = "ns:item/sword"
      generation {
        parent = "minecraft:item/handheld"
        textures { layer0 = "ns:item/sword" }
      }
    }
  }
}
emojis {
  "ns:hi" {
    keywords = [":)", ":hi:"]
    content = "x"
  }
}
"#;
        let b = BuildPk::parse(src).unwrap();
        assert!(b.contents["items"].contains_key("ns:sword"));
        let emoji = &b.contents["emojis"]["ns:hi"];
        assert!(matches!(emoji["keywords"], YamlValue::Sequence(_)));
    }

    #[test]
    fn variants_normalize_dot_keys_and_zip_level() {
        let src = r#"
project { name = "p" namespace = "ns" }
pack { packFormat = 34 }
export {
  resourcePackZip = "build/resource_pack.zip"
  variants {
    "26.2" {
      packFormat = 88
      resourcePackZip = "build/resource_pack_26_2.zip"
    }
    "26_1" {
      packFormat = 84
      resourcePackZip = "build/resource_pack_26_1.zip"
      overlays = ["26_1"]
    }
  }
  defaultVariants = ["26.2", "26_1"]
}
zip {
  method = DEFLATED
  level = 9
}
"#;
        let b = BuildPk::parse(src).unwrap();
        assert!(b.export.variants.contains_key("26_2"));
        assert_eq!(b.export.variants["26_2"].pack_format, 88);
        assert_eq!(b.export.variants["26_2"].min_format, [88, 0]);
        assert_eq!(b.export.variants["26_2"].supported_formats.max_inclusive, 88);
        assert_eq!(b.export.default_variants, vec!["26_2", "26_1"]);
        assert_eq!(b.zip.level, 9);
        let targets = b.resolve_pack_targets(&[]).unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].name, "26_2");
        assert_eq!(targets[0].pack_format, 88);
    }
}
