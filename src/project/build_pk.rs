//! Gradle-inspired `build.pk` DSL.
//!
//! ```text
//! project {
//!   name = "demo"
//!   namespace = "demo"
//!   version = "1.0.0"
//! }
//!
//! mappings {
//!   mode = WHOLE
//! }
//! ```

use crate::error::{Error, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildPk {
    pub project: ProjectMeta,
    pub mappings: MappingsConfig,
    pub pack: PackConfig,
    pub export: ExportConfig,
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
}

fn default_pack_format() -> u32 {
    34
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

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            supported_version: SupportedVersion::default(),
            features: FeatureFlags::default(),
            description: String::new(),
            pack_format: default_pack_format(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    #[serde(default = "default_pack_dir")]
    pub pack_dir: String,
    #[serde(default = "default_zip_out")]
    pub resource_pack_zip: String,
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
        }
    }
}

impl BuildPk {
    pub fn for_new_project(name: &str, namespace: &str) -> Self {
        let mut b = Self::default();
        b.project.name = name.to_string();
        b.project.namespace = namespace.to_string();
        b.pack.description = format!("<white>{name}</white>");
        b
    }

    pub fn render_dsl(&self) -> String {
        let f = &self.pack.features;
        let mut overrides = String::new();
        for (k, v) in &self.mappings.font.overrides {
            overrides.push_str(&format!(
                "      override(\"{k}\", {v})\n"
            ));
        }
        let mut cmd_ov = String::new();
        for (k, v) in &self.mappings.custom_model_data.overrides {
            cmd_ov.push_str(&format!("      override(\"{k}\", {v})\n"));
        }
        format!(
            r#"// PackCreator build script (Gradle-inspired)
// Project root MUST contain: build.pk + src/main/

project {{
  name = "{name}"
  namespace = "{ns}"
  version = "{ver}"
  author = "{author}"
  description = "{desc}"
  enable = {enable}
}}

mappings {{
  // WHOLE = embed full block_state_mappings table on pack export
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
}}
"#,
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
        )
    }

    pub fn parse(source: &str) -> Result<Self> {
        parse_build_pk(source)
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Debug, Clone)]
enum Value {
    String(String),
    Int(i64),
    Bool(bool),
    Ident(String),
    Block(IndexMap<String, Value>),
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

    fn parse_value(&mut self) -> Result<Value> {
        self.skip_ws_comments();
        match self.peek() {
            Some('"') => Ok(Value::String(self.parse_string()?)),
            Some('{') => Ok(Value::Block(self.parse_block_body()?)),
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let start = self.i;
                if c == '-' {
                    self.bump();
                }
                while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                    self.bump();
                }
                let n: i64 = self.src[start..self.i]
                    .parse()
                    .map_err(|e| Error::BuildPk(format!("bad int: {e}")))?;
                Ok(Value::Int(n))
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
            let key = self.expect_ident()?;
            self.skip_ws_comments();
            if self.peek() == Some('{') {
                let body = Value::Block(self.parse_block_body()?);
                map.insert(key, body);
                continue;
            }
            if self.peek() == Some('(') {
                // statement call: override("x", 1)
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

fn block<'a>(v: &'a Value) -> Result<&'a IndexMap<String, Value>> {
    match v {
        Value::Block(m) => Ok(m),
        _ => Err(Error::BuildPk("expected block".into())),
    }
}

fn parse_build_pk(source: &str) -> Result<BuildPk> {
    let mut p = Parser::new(source);
    let root = p.parse_file()?;
    let mut out = BuildPk::default();
    // Force explicit project identity; defaults are only for generated scaffolds.
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
            for (k, v) in f {
                if let Value::Call { name, args } = v {
                    if name == "override" && args.len() == 2 {
                        out.mappings
                            .font
                            .overrides
                            .insert(as_str(&args[0])?, as_u32(&args[1])?);
                    }
                }
                let _ = k;
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
        if let Some(v) = m.get("packFormat") {
            out.pack.pack_format = as_u32(v)?;
        }
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
        assert!(parsed.pack.features.images);
    }

    #[test]
    fn parse_feature_flags_and_export_paths() {
        let src = r#"
project {
  name = "p"
  namespace = "ns"
  version = "2.0.0"
  enable = false
}
mappings {
  mode = CUSTOM
  font {
    codepointStartingValue = 100
    offsetCharacters = false
    offsetFont = "minecraft:alt"
    override("minecraft:default", 60000)
  }
  customModelData {
    startingValue = 42
    override("PAPER", 99)
  }
}
pack {
  packFormat = 15
  description = "hi"
  supportedVersion {
    min = "1.19"
    max = "1.21"
  }
  features {
    images = true
    items = false
    blocks = false
    lootTables = false
  }
}
export {
  packDir = "out/pack"
  resourcePackZip = "out/pack.zip"
}
"#;
        let b = BuildPk::parse(src).unwrap();
        assert!(!b.project.enable);
        assert_eq!(b.mappings.mode, MappingsMode::Custom);
        assert_eq!(b.mappings.font.codepoint_starting_value, 100);
        assert!(!b.mappings.font.offset_characters);
        assert_eq!(
            b.mappings.font.overrides.get("minecraft:default"),
            Some(&60_000)
        );
        assert_eq!(b.mappings.custom_model_data.starting_value, 42);
        assert_eq!(
            b.mappings.custom_model_data.overrides.get("PAPER"),
            Some(&99)
        );
        assert_eq!(b.pack.pack_format, 15);
        assert_eq!(b.pack.supported_version.min, "1.19");
        assert!(b.pack.features.images);
        assert!(!b.pack.features.items);
        assert!(!b.pack.features.blocks);
        assert!(!b.pack.features.loot_tables);
        assert_eq!(b.export.pack_dir, "out/pack");
        assert_eq!(b.export.resource_pack_zip, "out/pack.zip");
    }

    #[test]
    fn parse_rejects_unknown_mappings_mode() {
        let src = r#"
project { name = "a" namespace = "a" }
mappings { mode = NOPE }
"#;
        let err = BuildPk::parse(src).unwrap_err();
        assert!(err.to_string().contains("unknown mappings mode"));
    }

    #[test]
    fn comments_are_ignored() {
        let src = r#"
// leading comment
project {
  // inside
  name = "c"
  namespace = "c"
}
"#;
        let b = BuildPk::parse(src).unwrap();
        assert_eq!(b.project.name, "c");
    }
}
