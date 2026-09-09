//! Client-layer Component baking for resource-pack lang strings.
//!
//! Authors write MiniMessage-like text (`<image:…>`, `<shift:N>`, colors).
//! At pack time we resolve glyphs to PUA codepoints + `§` legacy codes so
//! Minecraft client lang JSON (plain strings) can show custom font glyphs.
//! This mirrors CraftEngine's LangParser → Legacy serialize path.

use crate::error::{Error, Result};
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;

/// One bitmap image's assigned characters (row-major grid).
#[derive(Debug, Clone)]
pub struct ImageGlyph {
    #[allow(dead_code)]
    pub font: String,
    pub rows: Vec<Vec<char>>,
}

impl ImageGlyph {
    pub fn at(&self, row: usize, col: usize) -> Option<char> {
        self.rows.get(row)?.get(col).copied()
    }
}

/// Binary-power + small offsets for `<shift:N>` (from offset_chars.yml).
#[derive(Debug, Clone)]
pub struct OffsetFont {
    #[allow(dead_code)]
    pub font: String,
    pub neg: BTreeMap<i32, char>,
    pub pos: BTreeMap<i32, char>,
}

impl OffsetFont {
    pub fn empty(font: impl Into<String>) -> Self {
        Self {
            font: font.into(),
            neg: BTreeMap::new(),
            pos: BTreeMap::new(),
        }
    }

    pub fn from_offset_chars_yaml(font: &str, yaml: &str) -> Result<Self> {
        let root: YamlValue = serde_yaml::from_str(yaml)?;
        let mut out = Self::empty(font);
        let Some(images) = root
            .as_mapping()
            .and_then(|m| m.get(YamlValue::String("images".into())))
            .and_then(|v| v.as_mapping())
        else {
            return Ok(out);
        };
        for (id, value) in images {
            let Some(id) = id.as_str() else { continue };
            let Some(map) = value.as_mapping() else { continue };
            let ch = map
                .get(YamlValue::String("char".into()))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let decoded = decode_char_token(ch);
            let Some(c) = decoded.chars().next() else {
                continue;
            };
            if let Some(n) = id.strip_prefix("internal:neg_") {
                if let Ok(v) = n.parse::<i32>() {
                    out.neg.insert(v, c);
                }
            } else if let Some(n) = id.strip_prefix("internal:pos_") {
                if let Ok(v) = n.parse::<i32>() {
                    out.pos.insert(v, c);
                }
            }
        }
        Ok(out)
    }

    /// Encode a pixel shift as raw offset characters (no font tags — for client lang).
    pub fn encode_raw(&self, offset: i32) -> Result<String> {
        if offset == 0 {
            return Ok(String::new());
        }
        if offset > 0 {
            self.encode_pos(offset as u32)
        } else {
            self.encode_neg((-offset) as u32)
        }
    }

    fn encode_pos(&self, mut offset: u32) -> Result<String> {
        let mut s = String::new();
        while offset >= 256 {
            s.push(self.need_pos(256)?);
            offset -= 256;
        }
        for power in [128u32, 64, 48, 32, 16] {
            if offset >= power {
                s.push(self.need_pos(power)?);
                offset -= power;
            }
        }
        if offset > 0 {
            s.push(self.need_pos(offset)?);
        }
        Ok(s)
    }

    fn encode_neg(&self, mut offset: u32) -> Result<String> {
        let mut s = String::new();
        while offset >= 256 {
            s.push(self.need_neg(256)?);
            offset -= 256;
        }
        for power in [128u32, 64, 48, 32, 16] {
            if offset >= power {
                s.push(self.need_neg(power)?);
                offset -= power;
            }
        }
        if offset > 0 {
            s.push(self.need_neg(offset)?);
        }
        Ok(s)
    }

    fn need_pos(&self, n: u32) -> Result<char> {
        self.pos
            .get(&(n as i32))
            .copied()
            .ok_or_else(|| Error::Pack(format!("missing positive offset glyph pos_{n}")))
    }

    fn need_neg(&self, n: u32) -> Result<char> {
        self.neg
            .get(&(n as i32))
            .copied()
            .ok_or_else(|| Error::Pack(format!("missing negative offset glyph neg_{n}")))
    }
}

#[derive(Debug, Clone, Default)]
pub struct GlyphRegistry {
    pub images: BTreeMap<String, ImageGlyph>,
    pub offsets: Option<OffsetFont>,
}

impl GlyphRegistry {
    pub fn resolve_image(&self, spec: &str) -> Result<char> {
        // formats: ns:id | ns:id:row:col
        let parts: Vec<&str> = spec.split(':').collect();
        let (id, row, col) = match parts.as_slice() {
            [ns, name] => (format!("{ns}:{name}"), 0usize, 0usize),
            [ns, name, r, c] => {
                let row: usize = r.parse().map_err(|_| {
                    Error::Pack(format!("bad image row in <image:{spec}>"))
                })?;
                let col: usize = c.parse().map_err(|_| {
                    Error::Pack(format!("bad image col in <image:{spec}>"))
                })?;
                (format!("{ns}:{name}"), row, col)
            }
            _ => {
                return Err(Error::Pack(format!(
                    "bad <image:{spec}>; expected ns:id or ns:id:row:col"
                )))
            }
        };
        let glyph = self.images.get(&id).ok_or_else(|| {
            Error::Pack(format!("unknown image `{id}` referenced by <image:{spec}>"))
        })?;
        glyph.at(row, col).ok_or_else(|| {
            Error::Pack(format!(
                "image `{id}` has no glyph at {row}:{col} (grid {}x{})",
                glyph.rows.len(),
                glyph.rows.first().map(|r| r.len()).unwrap_or(0)
            ))
        })
    }
}

/// Bake MiniMessage-like author text into a Minecraft client lang string.
pub fn bake_client_lang(input: &str, glyphs: &GlyphRegistry) -> Result<String> {
    let mut s = legacy_ampersand_to_section(input);
    // Strip no-op / common MiniMessage resets that have no § equivalent beyond §r
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            if let Some((tag, next)) = read_tag(&chars, i) {
                i = next;
                if let Some(replacement) = expand_tag(&tag, glyphs)? {
                    out.push_str(&replacement);
                }
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    let _ = &mut s;
    Ok(out)
}

fn legacy_ampersand_to_section(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if let Some(n) = chars.peek().copied() {
                if is_legacy_code(n) {
                    out.push('§');
                    out.push(chars.next().unwrap());
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

fn is_legacy_code(c: char) -> bool {
    matches!(
        c,
        '0'..='9'
            | 'a'..='f'
            | 'A'..='F'
            | 'k'..='o'
            | 'K'..='O'
            | 'r'
            | 'R'
            | 'x'
            | 'X'
    )
}

fn read_tag(chars: &[char], start: usize) -> Option<(String, usize)> {
    // start at '<'
    let mut i = start + 1;
    let mut depth = 1;
    let mut tag = String::new();
    while i < chars.len() {
        let c = chars[i];
        if c == '<' {
            depth += 1;
            tag.push(c);
            i += 1;
            continue;
        }
        if c == '>' {
            depth -= 1;
            if depth == 0 {
                return Some((tag, i + 1));
            }
            tag.push(c);
            i += 1;
            continue;
        }
        tag.push(c);
        i += 1;
    }
    None
}

fn expand_tag(tag: &str, glyphs: &GlyphRegistry) -> Result<Option<String>> {
    let tag = tag.trim();
    if tag.is_empty() {
        return Ok(Some(String::new()));
    }

    // closing tags
    if let Some(rest) = tag.strip_prefix('/') {
        return Ok(Some(close_tag(rest.trim())));
    }

    // self-closing style: image:… / shift:…
    let (name, arg) = split_tag_name(tag);
    let name_l = name.to_ascii_lowercase();

    match name_l.as_str() {
        "image" => {
            let spec = arg.trim().trim_matches('"').trim_matches('\'');
            if spec.is_empty() {
                return Err(Error::Pack("<image:> missing id".into()));
            }
            let ch = glyphs.resolve_image(spec)?;
            Ok(Some(ch.to_string()))
        }
        "shift" => {
            let n: i32 = arg.trim().parse().map_err(|_| {
                Error::Pack(format!("bad <shift:{arg}>; expected integer"))
            })?;
            let offsets = glyphs.offsets.as_ref().ok_or_else(|| {
                Error::Pack(
                    "<shift:> used but mappings.font.offsetCharacters is disabled / missing"
                        .into(),
                )
            })?;
            Ok(Some(offsets.encode_raw(n)?))
        }
        "reset" | "r" => Ok(Some("§r".into())),
        "bold" | "b" => Ok(Some("§l".into())),
        "italic" | "i" | "em" => Ok(Some("§o".into())),
        "underlined" | "u" => Ok(Some("§n".into())),
        "strikethrough" | "st" => Ok(Some("§m".into())),
        "obfuscated" | "obf" => Ok(Some("§k".into())),
        // MiniMessage <!italic> / <!i> = not italic → often used as reset italic; use §r then continue
        "!italic" | "!i" | "!em" => Ok(Some("§r".into())),
        "!bold" | "!b" => Ok(Some("§r".into())),
        // named colors
        "black" => Ok(Some("§0".into())),
        "dark_blue" => Ok(Some("§1".into())),
        "dark_green" => Ok(Some("§2".into())),
        "dark_aqua" => Ok(Some("§3".into())),
        "dark_red" => Ok(Some("§4".into())),
        "dark_purple" => Ok(Some("§5".into())),
        "gold" => Ok(Some("§6".into())),
        "gray" | "grey" => Ok(Some("§7".into())),
        "dark_gray" | "dark_grey" => Ok(Some("§8".into())),
        "blue" => Ok(Some("§9".into())),
        "green" => Ok(Some("§a".into())),
        "aqua" => Ok(Some("§b".into())),
        "red" => Ok(Some("§c".into())),
        "light_purple" | "pink" => Ok(Some("§d".into())),
        "yellow" => Ok(Some("§e".into())),
        "white" => Ok(Some("§f".into())),
        // hex <#RRGGBB> or <color:#RRGGBB>
        "color" => Ok(Some(hex_to_legacy(arg.trim())?)),
        other if other.starts_with('#') => Ok(Some(hex_to_legacy(other)?)),
        // unknown: strip tag (warn via empty) — keep author text readable
        _ => Ok(Some(String::new())),
    }
}

fn close_tag(name: &str) -> String {
    let n = name.to_ascii_lowercase();
    match n.as_str() {
        "bold" | "b" | "italic" | "i" | "em" | "underlined" | "u" | "strikethrough" | "st"
        | "obfuscated" | "obf" | "black" | "dark_blue" | "dark_green" | "dark_aqua"
        | "dark_red" | "dark_purple" | "gold" | "gray" | "grey" | "dark_gray" | "dark_grey"
        | "blue" | "green" | "aqua" | "red" | "light_purple" | "pink" | "yellow" | "white"
        | "color" => "§r".into(),
        _ if n.starts_with('#') => "§r".into(),
        _ => String::new(),
    }
}

fn split_tag_name(tag: &str) -> (&str, &str) {
    if let Some((n, a)) = tag.split_once(':') {
        (n.trim(), a.trim())
    } else if let Some((n, a)) = tag.split_once(' ') {
        (n.trim(), a.trim())
    } else {
        (tag.trim(), "")
    }
}

fn hex_to_legacy(hex: &str) -> Result<String> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::Pack(format!("bad hex color `{hex}`")));
    }
    // §x§R§R§G§G§B§B
    let mut out = String::from("§x");
    for c in h.chars() {
        out.push('§');
        out.push(c.to_ascii_lowercase());
    }
    Ok(out)
}

fn decode_char_token(s: &str) -> String {
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
    if let Some(hex) = s.strip_prefix('u').or_else(|| s.strip_prefix("U+")) {
        if let Ok(cp) = u32::from_str_radix(hex, 16) {
            if let Some(ch) = char::from_u32(cp) {
                return ch.to_string();
            }
        }
    }
    s.to_string()
}

/// Flatten nested YAML mappings into dotted keys (`a.b.c`).
#[allow(dead_code)]
pub fn flatten_lang_value(prefix: Option<&str>, value: &YamlValue, out: &mut BTreeMap<String, String>) {
    match value {
        YamlValue::Mapping(m) => {
            for (k, v) in m {
                let Some(ks) = k.as_str() else { continue };
                let key = match prefix {
                    Some(p) => format!("{p}.{ks}"),
                    None => ks.to_string(),
                };
                flatten_lang_value(Some(&key), v, out);
            }
        }
        YamlValue::String(s) => {
            if let Some(p) = prefix {
                out.insert(p.to_string(), s.clone());
            }
        }
        YamlValue::Number(n) => {
            if let Some(p) = prefix {
                out.insert(p.to_string(), n.to_string());
            }
        }
        YamlValue::Bool(b) => {
            if let Some(p) = prefix {
                out.insert(p.to_string(), b.to_string());
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with_icon() -> GlyphRegistry {
        let mut images = BTreeMap::new();
        images.insert(
            "demo:icon".into(),
            ImageGlyph {
                font: "minecraft:default".into(),
                rows: vec![vec!['\u{E001}']],
            },
        );
        let offsets = OffsetFont {
            font: "minecraft:default".into(),
            neg: BTreeMap::from([(1, '\u{F800}'), (2, '\u{F801}'), (16, '\u{F80F}')]),
            pos: BTreeMap::from([(1, '\u{F830}'), (16, '\u{F83F}')]),
        };
        GlyphRegistry {
            images,
            offsets: Some(offsets),
        }
    }

    #[test]
    fn bakes_image_and_color() {
        let g = registry_with_icon();
        let out = bake_client_lang("<white><image:demo:icon> Hi</white>", &g).unwrap();
        assert!(out.contains('\u{E001}'));
        assert!(out.starts_with("§f"));
        assert!(out.contains("Hi"));
        assert!(out.contains('§'));
    }

    #[test]
    fn bakes_shift() {
        let g = registry_with_icon();
        let out = bake_client_lang("<shift:-18>", &g).unwrap();
        // -18 = neg_16 + neg_2
        assert_eq!(out, "\u{F80F}\u{F801}");
    }

    #[test]
    fn ampersand_legacy() {
        let g = GlyphRegistry::default();
        let out = bake_client_lang("&aHello &r", &g).unwrap();
        assert_eq!(out, "§aHello §r");
    }
}
