use crate::error::{Error, Result};
use crate::project::{BuildPk, MappingsMode};
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Stable ID allocator with JSON cache (font codepoints / CMD).
#[derive(Debug)]
pub struct IdAllocator {
    cache_path: PathBuf,
    next: u32,
    assigned: BTreeMap<String, u32>,
    pending: BTreeMap<String, Option<u32>>,
    used: BTreeSet<u32>,
}

impl IdAllocator {
    pub fn new(cache_path: PathBuf, starting: u32) -> Self {
        Self {
            cache_path,
            next: starting,
            assigned: BTreeMap::new(),
            pending: BTreeMap::new(),
            used: BTreeSet::new(),
        }
    }

    pub fn load(&mut self) -> Result<()> {
        if !self.cache_path.is_file() {
            return Ok(());
        }
        let text = fs::read_to_string(&self.cache_path)?;
        let map: BTreeMap<String, u32> = serde_json::from_str(&text)?;
        for (k, v) in map {
            self.used.insert(v);
            if v >= self.next {
                self.next = v + 1;
            }
            self.assigned.insert(k, v);
        }
        Ok(())
    }

    pub fn request_auto(&mut self, name: &str) -> u32 {
        if let Some(v) = self.assigned.get(name) {
            return *v;
        }
        if let Some(Some(v)) = self.pending.get(name) {
            return *v;
        }
        while self.used.contains(&self.next) {
            self.next += 1;
        }
        let id = self.next;
        self.next += 1;
        self.used.insert(id);
        self.pending.insert(name.to_string(), Some(id));
        id
    }

    pub fn assign_fixed(&mut self, name: &str, id: u32) -> Result<u32> {
        if let Some(existing) = self.assigned.get(name) {
            if *existing != id {
                return Err(Error::Pack(format!(
                    "id conflict for {name}: cached {existing} vs fixed {id}"
                )));
            }
            return Ok(id);
        }
        if self.used.contains(&id) {
            return Err(Error::Pack(format!(
                "id {id} already used, cannot assign to {name}"
            )));
        }
        self.used.insert(id);
        self.pending.insert(name.to_string(), Some(id));
        Ok(id)
    }

    pub fn process_pending(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        for (k, v) in pending {
            if let Some(id) = v {
                self.assigned.insert(k, id);
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(&self.assigned)?;
        fs::write(&self.cache_path, text)?;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<u32> {
        self.assigned.get(name).copied()
    }

    pub fn all(&self) -> &BTreeMap<String, u32> {
        &self.assigned
    }
}

pub struct MappingBundle {
    pub block_state_mappings: Value,
    pub font_allocators: BTreeMap<String, IdAllocator>,
    pub cmd_allocators: BTreeMap<String, IdAllocator>,
}

pub fn embedded_whole_mappings_yaml() -> &'static str {
    include_str!("../../assets/mappings.whole.yml")
}

pub fn load_whole_block_state_mappings() -> Result<Value> {
    let v: Value = serde_yaml::from_str(embedded_whole_mappings_yaml())?;
    Ok(v)
}

pub fn prepare_mappings(build: &BuildPk, cache_root: &Path) -> Result<MappingBundle> {
    let block_state_mappings = match build.mappings.mode {
        MappingsMode::Whole => load_whole_block_state_mappings()?,
        MappingsMode::Custom => Value::Mapping(Default::default()),
    };

    let mut font_allocators = BTreeMap::new();
    // default allocator keyed by font name; created lazily by packer
    let _ = cache_root;
    let mut cmd_allocators = BTreeMap::new();
    let _ = (&mut font_allocators, &mut cmd_allocators);

    Ok(MappingBundle {
        block_state_mappings,
        font_allocators,
        cmd_allocators,
    })
}

pub fn font_starting_value(build: &BuildPk, font: &str) -> u32 {
    build
        .mappings
        .font
        .overrides
        .get(font)
        .copied()
        .unwrap_or(build.mappings.font.codepoint_starting_value)
}

pub fn cmd_starting_value(build: &BuildPk, material: &str) -> u32 {
    build
        .mappings
        .custom_model_data
        .overrides
        .get(material)
        .copied()
        .unwrap_or(build.mappings.custom_model_data.starting_value)
}

pub fn font_allocator(build: &BuildPk, cache_root: &Path, font: &str) -> Result<IdAllocator> {
    let safe = font.replace(':', "__");
    let path = cache_root.join("font").join(format!("{safe}.json"));
    let mut alloc = IdAllocator::new(path, font_starting_value(build, font));
    alloc.load()?;
    Ok(alloc)
}

pub fn cmd_allocator(build: &BuildPk, cache_root: &Path, material: &str) -> Result<IdAllocator> {
    let path = cache_root
        .join("custom_model_data")
        .join(format!("{material}.json"));
    let mut alloc = IdAllocator::new(path, cmd_starting_value(build, material));
    alloc.load()?;
    Ok(alloc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_cache(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pc-alloc-{name}-{nanos}.json"))
    }

    #[test]
    fn auto_ids_are_stable_across_save_load() {
        let path = tmp_cache("auto");
        let mut a = IdAllocator::new(path.clone(), 100);
        let id1 = a.request_auto("img:0:0");
        let id2 = a.request_auto("img:0:1");
        assert_eq!(id1, 100);
        assert_eq!(id2, 101);
        a.process_pending();
        a.save().unwrap();

        let mut b = IdAllocator::new(path.clone(), 100);
        b.load().unwrap();
        assert_eq!(b.request_auto("img:0:0"), 100);
        assert_eq!(b.request_auto("img:0:1"), 101);
        assert_eq!(b.request_auto("img:1:0"), 102);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn fixed_id_conflict_is_rejected() {
        let path = tmp_cache("fixed");
        let mut a = IdAllocator::new(path.clone(), 1);
        a.assign_fixed("a", 5).unwrap();
        a.process_pending();
        a.save().unwrap();

        let mut b = IdAllocator::new(path.clone(), 1);
        b.load().unwrap();
        assert!(b.assign_fixed("a", 6).is_err());
        assert!(b.assign_fixed("b", 5).is_err());
        assert_eq!(b.assign_fixed("a", 5).unwrap(), 5);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn whole_mappings_yaml_parses_and_has_block_state_root() {
        let v = load_whole_block_state_mappings().unwrap();
        let map = v.as_mapping().unwrap();
        assert!(map.contains_key(Value::String("block_state_mappings".into())));
        let entries = map
            .get(Value::String("block_state_mappings".into()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert!(entries.len() > 100);
    }

    #[test]
    fn font_and_cmd_starting_value_respect_overrides() {
        let mut build = BuildPk::for_new_project("t", "t");
        build
            .mappings
            .font
            .overrides
            .insert("minecraft:default".into(), 57_344);
        build
            .mappings
            .custom_model_data
            .overrides
            .insert("PAPER".into(), 12_000);
        assert_eq!(font_starting_value(&build, "minecraft:default"), 57_344);
        assert_eq!(
            font_starting_value(&build, "other:font"),
            build.mappings.font.codepoint_starting_value
        );
        assert_eq!(cmd_starting_value(&build, "PAPER"), 12_000);
        assert_eq!(
            cmd_starting_value(&build, "STONE"),
            build.mappings.custom_model_data.starting_value
        );
    }
}
