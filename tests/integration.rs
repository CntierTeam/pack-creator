//! Integration: create Project → check → build → assert pack layout + zip.

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

    // All configuration aggregates in build.pk (not scattered YAML scaffolds)
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
        "entity_models",
    ] {
        assert!(
            build.contents.contains_key(required),
            "missing build.pk section {required}; have {:?}",
            build.contents.keys().collect::<Vec<_>>()
        );
    }
    assert!(build.contents["items"].contains_key("demopack:demo_item"));
    assert!(build.contents["images"].contains_key("demopack:main_gui"));

    let text = fs::read_to_string(Project::build_pk_path(&root)).unwrap();
    assert!(text.contains("items {"));
    assert!(text.contains("\"demopack:demo_item\""));
    assert!(!Project::configuration_dir(&root).join("images.yml").is_file());

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

    assert!(
        names
            .iter()
            .any(|n| n == "assets/fullpk/items/demo_item.json"),
        "missing modern item model in {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n == "assets/fullpk/models/item/demo_item.json"),
        "missing generated item model json in {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n == "assets/minecraft/models/item/paper.json"),
        "missing legacy CMD override file in {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n == "assets/minecraft/font/gui.json"),
        "missing GUI font in {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n == "assets/fullpk/models/entity/demo_cow.json"),
        "missing entity model in {names:?}"
    );
    assert!(
        names
            .iter()
            .any(|n| n == "assets/minecraft/textures/entity/cow/cow.png"),
        "missing entity texture replacement in {names:?}"
    );
    assert!(report.item_models >= 1);
    assert!(report.modern_items >= 1);
    assert!(report.legacy_override_files >= 1);
    assert!(report.entity_models >= 1);
    assert!(report.entity_texture_replacements >= 1);

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
fn zip_pack_mcmeta_has_modern_format_fields() {
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
    assert_eq!(
        json["pack"]["supported_formats"]["min_inclusive"]
            .as_u64()
            .unwrap(),
        u64::from(project.build.pack.supported_formats.min_inclusive)
    );
    assert_eq!(
        json["pack"]["min_format"][0].as_u64().unwrap(),
        u64::from(project.build.pack.min_format[0])
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn multi_variant_writes_separate_zips() {
    let root = unique_dir("multi-var");
    let project = Project::create(&root, "Multi", "multi").unwrap();
    let mut build = project.build.clone();
    build.export.variants.insert(
        "26_1".into(),
        pack_creator::project::VariantExport::from_pack_format(
            84,
            "build/resource_pack_26_1.zip",
        ),
    );
    build.export.variants.insert(
        "26_2".into(),
        pack_creator::project::VariantExport::from_pack_format(
            88,
            "build/resource_pack_26_2.zip",
        ),
    );
    build.export.default_variants = vec!["26_1".into(), "26_2".into()];
    build.zip.level = 1;
    fs::write(Project::build_pk_path(&root), build.render_dsl()).unwrap();

    let project = Project::open(&root).unwrap();
    let report = build_project(&project).unwrap();
    assert_eq!(report.resource_pack_zips.len(), 2);
    assert!(root.join("build/resource_pack_26_1.zip").is_file());
    assert!(root.join("build/resource_pack_26_2.zip").is_file());

    for (name, expect_fmt) in [("26_1", 84u64), ("26_2", 88u64)] {
        let zip = root.join(format!("build/resource_pack_{name}.zip"));
        let f = fs::File::open(&zip).unwrap();
        let mut archive = ZipArchive::new(f).unwrap();
        let mut entry = archive.by_name("pack.mcmeta").unwrap();
        let mut text = String::new();
        entry.read_to_string(&mut text).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["pack"]["pack_format"].as_u64().unwrap(), expect_fmt);
        assert_eq!(
            json["pack"]["max_format"][0].as_u64().unwrap(),
            expect_fmt
        );
    }

    let filtered =
        pack_creator::build_project_filtered(&project, &["26.2".into()]).unwrap();
    assert_eq!(filtered.variants_built, vec!["26_2".to_string()]);
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
