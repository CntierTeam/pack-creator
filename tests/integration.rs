//! Integration: create Project → check → build → assert pack layout + zip.

use pack_creator::config::scan_configuration;
use pack_creator::mapping::embedded_whole_mappings_yaml;
use pack_creator::project::{MappingsMode, Project};
use pack_creator::{build_project, BuildPk};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use zip::ZipArchive;

fn unique_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("packcreator-{prefix}-{nanos}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn zip_names(zip_path: &Path) -> Vec<String> {
    let f = fs::File::open(zip_path).unwrap();
    let mut archive = ZipArchive::new(f).unwrap();
    let mut names = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).unwrap();
        names.push(entry.name().to_string());
    }
    names.sort();
    names
}

#[test]
fn create_requires_empty_directory() {
    let root = unique_dir("nonempty");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("junk.txt"), "x").unwrap();
    let err = Project::create(&root, "x", "x").unwrap_err();
    assert!(err.to_string().contains("not empty"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn create_scaffolds_required_layout() {
    let root = unique_dir("scaffold");
    let project = Project::create(&root, "DemoPack", "demopack").unwrap();

    assert!(Project::build_pk_path(&root).is_file());
    assert!(Project::src_main(&root).is_dir());
    assert!(Project::configuration_dir(&root).is_dir());
    assert!(Project::resourcepack_dir(&root).is_dir());
    assert!(Project::src_main(&root).join("pack.yml").is_file());

    let build = &project.build;
    assert_eq!(build.project.name, "DemoPack");
    assert_eq!(build.project.namespace, "demopack");
    assert_eq!(build.mappings.mode, MappingsMode::Whole);

    let idx = scan_configuration(&Project::configuration_dir(&root)).unwrap();
    for required in [
        "images",
        "emojis",
        "lang",
        "sounds",
        "equipments",
        "items",
        "blocks",
        "furniture",
        "paintings",
        "templates",
        "global-variables",
        "recipes",
        "categories",
        "loot-tables",
    ] {
        assert!(
            idx.sections.contains_key(required),
            "missing section {required}; have {:?}",
            idx.sections.keys().collect::<Vec<_>>()
        );
    }

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn open_rejects_missing_build_pk() {
    let root = unique_dir("nobuild");
    fs::create_dir_all(Project::src_main(&root)).unwrap();
    let err = Project::open(&root).unwrap_err();
    assert!(err.to_string().contains("build.pk"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn build_exports_pack_dir_and_zip() {
    let root = unique_dir("build");
    let project = Project::create(&root, "FullPack", "fullpk").unwrap();
    let report = build_project(&project).unwrap();

    // pack dir
    assert!(report.pack_dir.join("pack.yml").is_file());
    assert!(report
        .pack_dir
        .join("configuration")
        .join("images.yml")
        .is_file());
    assert!(report
        .pack_dir
        .join("configuration")
        .join("block_state_mappings.yml")
        .is_file());
    assert!(report
        .pack_dir
        .join("configuration")
        .join("_pack_creator_mappings.yml")
        .is_file());
    assert!(report.pack_dir.join("resourcepack").is_dir());

    let mappings_text =
        fs::read_to_string(report.pack_dir.join("configuration/block_state_mappings.yml"))
            .unwrap();
    assert!(mappings_text.contains("block_state_mappings:"));
    assert_eq!(
        mappings_text.lines().count(),
        embedded_whole_mappings_yaml().lines().count()
    );

    let pack_yml = fs::read_to_string(report.pack_dir.join("pack.yml")).unwrap();
    assert!(pack_yml.contains("namespace: fullpk"));
    assert!(pack_yml.contains("enable: true"));

    // ZIP
    assert!(report.resource_pack_zip.is_file());
    let names = zip_names(&report.resource_pack_zip);
    assert!(
        names.iter().any(|n| n == "pack.mcmeta"),
        "missing pack.mcmeta in {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n == "assets/minecraft/font/default.json"),
        "missing font json in {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n == "assets/minecraft/lang/en_us.json"),
        "missing en_us lang in {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "assets/fullpk/sounds.json"),
        "missing sounds.json in {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n == "assets/fullpk/equipment/demo_trim.json"),
        "missing equipment json in {names:?}"
    );

    // report metadata
    assert!(report.fonts_written >= 1);
    assert!(report.langs_written >= 1);
    assert!(report.sounds_written >= 1);
    assert!(report.sections.len() >= 10);
    assert!(root.join("build/report.json").is_file());
    assert!(root.join("build/cache/font/minecraft__default.json").is_file());

    // font cache should contain auto-assigned codepoints for example image
    let cache: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("build/cache/font/minecraft__default.json")).unwrap())
            .unwrap();
    assert!(cache.as_object().unwrap().len() >= 1);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn rebuild_is_idempotent_for_font_codepoints() {
    let root = unique_dir("rebuild");
    let project = Project::create(&root, "Rebuild", "rebuild").unwrap();
    let _ = build_project(&project).unwrap();
    let cache_path = root.join("build/cache/font/minecraft__default.json");
    let first = fs::read_to_string(&cache_path).unwrap();
    let project = Project::open(&root).unwrap();
    let _ = build_project(&project).unwrap();
    let second = fs::read_to_string(&cache_path).unwrap();
    assert_eq!(first, second);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn custom_mappings_mode_skips_whole_block_table() {
    let root = unique_dir("custom-map");
    let project = Project::create(&root, "Custom", "custom").unwrap();
    let mut build = project.build.clone();
    build.mappings.mode = MappingsMode::Custom;
    fs::write(Project::build_pk_path(&root), build.render_dsl()).unwrap();
    let project = Project::open(&root).unwrap();
    assert_eq!(project.build.mappings.mode, MappingsMode::Custom);
    let report = build_project(&project).unwrap();
    assert!(
        !report
            .pack_dir
            .join("configuration/block_state_mappings.yml")
            .is_file()
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn zip_pack_mcmeta_has_pack_format() {
    let root = unique_dir("mcmeta");
    let project = Project::create(&root, "Meta", "meta").unwrap();
    let report = build_project(&project).unwrap();
    let f = fs::File::open(&report.resource_pack_zip).unwrap();
    let mut archive = ZipArchive::new(f).unwrap();
    let mut entry = archive.by_name("pack.mcmeta").unwrap();
    let mut text = String::new();
    entry.read_to_string(&mut text).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        json["pack"]["pack_format"].as_u64().unwrap(),
        u64::from(project.build.pack.pack_format)
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn build_pk_parse_errors_on_missing_project_name() {
    let src = r#"
project {
  namespace = "x"
  version = "1"
}
"#;
    let err = BuildPk::parse(src).unwrap_err();
    assert!(err.to_string().contains("required"));
}
