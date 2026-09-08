mod build_pk;

pub use build_pk::*;

use crate::error::{Error, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub build: BuildPk,
}

impl Project {
    pub fn build_pk_path(root: &Path) -> PathBuf {
        root.join("build.pk")
    }

    pub fn src_main(root: &Path) -> PathBuf {
        root.join("src").join("main")
    }

    pub fn configuration_dir(root: &Path) -> PathBuf {
        Self::src_main(root).join("configuration")
    }

    pub fn resourcepack_dir(root: &Path) -> PathBuf {
        Self::src_main(root).join("resourcepack")
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let build_path = Self::build_pk_path(&root);
        if !build_path.is_file() {
            return Err(Error::Project(format!(
                "missing build.pk under {}",
                root.display()
            )));
        }
        if !Self::src_main(&root).is_dir() {
            return Err(Error::Project(format!(
                "missing src/main under {}",
                root.display()
            )));
        }
        let text = fs::read_to_string(&build_path)?;
        let build = BuildPk::parse(&text)?;
        Ok(Self { root, build })
    }

    pub fn create(root: impl AsRef<Path>, name: &str, namespace: &str) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        if root.exists() {
            let mut rd = root.read_dir()?;
            if rd.next().is_some() {
                return Err(Error::Project(format!(
                    "directory not empty: {}",
                    root.display()
                )));
            }
        }
        fs::create_dir_all(Self::configuration_dir(&root))?;
        fs::create_dir_all(Self::resourcepack_dir(&root).join("assets"))?;

        let build = BuildPk::for_new_project(name, namespace);
        fs::write(Self::build_pk_path(&root), build.render_dsl())?;

        write_scaffold_files(&root, &build)?;
        Self::open(root)
    }

    pub fn save_build(&self) -> Result<()> {
        fs::write(Self::build_pk_path(&self.root), self.build.render_dsl())?;
        Ok(())
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("build").join("cache")
    }
}

fn write_scaffold_files(root: &Path, build: &BuildPk) -> Result<()> {
    let ns = &build.project.namespace;
    let conf = Project::configuration_dir(root);

    // pack meta is emitted on export; also keep a readable copy for authors
    let pack_yml = format!(
        "enable: {}\nnamespace: {}\nauthor: {}\nversion: {}\ndescription: {}\n",
        build.project.enable,
        build.project.namespace,
        build.project.author,
        build.project.version,
        build.project.description
    );
    fs::write(Project::src_main(root).join("pack.yml"), pack_yml)?;

    let images = format!(
        r#"# Bitmap font / GUI images
images:
  {ns}:example_icon:
    height: 16
    ascent: 12
    font: minecraft:default
    file: {ns}:font/example_icon.png
    grid_size: 1,1
"#
    );
    fs::write(conf.join("images.yml"), images)?;

    let emoji = format!(
        r#"# Emoji (depends on images)
emojis:
  {ns}:smile:
    image: {ns}:example_icon:0:0
    keywords:
      - ":)"
      - ":smile:"
    content: "<white><image:{ns}:example_icon:0:0></white>"
"#
    );
    fs::write(conf.join("emoji.yml"), emoji)?;

    let lang = format!(
        r#"lang:
  en_us:
    pack.{ns}.name: "{name}"
  zh_cn:
    pack.{ns}.name: "{name}"
"#,
        name = build.project.name
    );
    fs::write(conf.join("lang.yml"), lang)?;

    let templates = format!(
        r#"templates:
  {ns}:placeholder/basic:
    content: "<gray>${{text:-hello}}</gray>"

global-variables:
  {ns}:welcome: "<white>Welcome to {name}</white>"
"#,
        name = build.project.name
    );
    fs::write(conf.join("templates.yml"), templates)?;

    let sounds = format!(
        r#"sounds:
  {ns}:demo_click:
    replace: false
    subtitle: "subtitles.{ns}.demo_click"
    sounds:
      - {ns}:ui/click
"#
    );
    fs::write(conf.join("sounds.yml"), sounds)?;

    let equipment = format!(
        r#"# Equipment assets (no item model override by itself)
equipments:
  {ns}:demo_trim:
    type: component
    humanoid: {ns}:entity/equipment/humanoid/demo
    humanoid_leggings: {ns}:entity/equipment/humanoid_leggings/demo
"#
    );
    fs::write(conf.join("equipment.yml"), equipment)?;

    let paintings = format!(
        r#"paintings:
  {ns}:demo_art:
    width: 1
    height: 1
    asset_id: {ns}:demo_art
    title: "<white>Demo Art</white>"
    author: "<gray>PackCreator</gray>"
"#
    );
    fs::write(conf.join("paintings.yml"), paintings)?;

    let items = format!(
        r#"# Custom items — generation writes models/*.json + items/*.json + CMD overrides
items:
  {ns}:demo_item:
    material: PAPER
    data:
      display-name: "<!i><white>Demo Item</white>"
    model:
      type: minecraft:model
      path: {ns}:item/demo_item
      generation:
        parent: minecraft:item/generated
        textures:
          layer0: {ns}:item/demo_item
"#
    );
    fs::write(conf.join("items.yml"), items)?;

    let gui = format!(
        r#"# GUI = bitmap images on font minecraft:gui (+ optional icon items)
# Textures go under resourcepack/assets/<ns>/textures/font/gui/
images:
  {ns}:main_gui:
    height: 140
    ascent: 18
    font: minecraft:gui
    file: {ns}:font/gui/main_gui.png

items:
  {ns}:gui_next:
    material: PAPER
    model:
      type: minecraft:model
      path: {ns}:item/gui/next
      generation:
        parent: minecraft:item/generated
        textures:
          layer0: {ns}:item/gui/next
"#
    );
    fs::write(conf.join("gui.yml"), gui)?;

    let entities = format!(
        r#"# Entity model / texture replacement
# - model: writes assets/<ns>/models/entity/*.json
# - replace_textures: copies PNG from this Project's resourcepack into the pack zip
entity_models:
  {ns}:demo_cow:
    model:
      path: {ns}:entity/demo_cow
      parent: minecraft:block/block
      textures:
        all: {ns}:entity/demo_cow
    replace_textures:
      - from: {ns}:entity/demo_cow
        to: minecraft:entity/cow/cow.png
"#
    );
    fs::write(conf.join("entity_models.yml"), entities)?;

    let blocks = format!(
        r#"# Custom blocks; WHOLE block_state_mappings come from build.pk
blocks:
  {ns}:demo_block:
    settings:
      hardness: 1.5
      resistance: 1.5
    state:
      auto_state: solid
      model:
        type: minecraft:model
        path: {ns}:block/demo_block
"#
    );
    fs::write(conf.join("blocks.yml"), blocks)?;

    let furniture = format!(
        r#"# Furniture uses item models in the client zip (ItemDisplay at runtime)
furniture:
  {ns}:demo_chair:
    settings:
      item: {ns}:demo_item
"#
    );
    fs::write(conf.join("furniture.yml"), furniture)?;

    let recipes = format!(
        r#"recipes:
  {ns}:demo_craft:
    type: shaped
    pattern:
      - " A "
      - " A "
      - " B "
    ingredients:
      A: PAPER
      B: STICK
    result:
      id: {ns}:demo_item
      count: 1
"#
    );
    fs::write(conf.join("recipes.yml"), recipes)?;

    let categories = format!(
        r#"categories:
  {ns}:demo:
    name: "<white>Demo</white>"
    icon: {ns}:demo_item
    list:
      - {ns}:demo_item
"#
    );
    fs::write(conf.join("categories.yml"), categories)?;

    let loot = format!(
        r#"loot-tables:
  {ns}:demo_drop:
    pools:
      - rolls: 1
        entries:
          - type: item
            item: {ns}:demo_item
            weight: 1
"#
    );
    fs::write(conf.join("loot_tables.yml"), loot)?;

    // placeholder texture note
    let tex_dir = Project::resourcepack_dir(root)
        .join("assets")
        .join(ns)
        .join("textures")
        .join("font");
    fs::create_dir_all(&tex_dir)?;
    fs::write(
        tex_dir.join("README.txt"),
        "Place example_icon.png here (referenced by configuration/images.yml).\n",
    )?;

    let gui_tex = Project::resourcepack_dir(root)
        .join("assets")
        .join(ns)
        .join("textures")
        .join("font")
        .join("gui");
    fs::create_dir_all(&gui_tex)?;
    fs::write(
        gui_tex.join("README.txt"),
        "Place main_gui.png here (referenced by configuration/gui.yml).\n",
    )?;

    let item_tex = Project::resourcepack_dir(root)
        .join("assets")
        .join(ns)
        .join("textures")
        .join("item");
    fs::create_dir_all(item_tex.join("gui"))?;
    fs::write(
        item_tex.join("README.txt"),
        "Place demo_item.png and gui/next.png textures referenced by items/gui config.\n",
    )?;

    let ent_tex = Project::resourcepack_dir(root)
        .join("assets")
        .join(ns)
        .join("textures")
        .join("entity");
    fs::create_dir_all(&ent_tex)?;
    fs::write(
        ent_tex.join("README.txt"),
        "Place demo_cow.png for entity_models replace_textures / model textures.\n",
    )?;

    // Minimal 1x1 PNGs so scaffold projects build without missing textures.
    let tiny_png: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x05, 0xFE, 0xD4, 0xEF, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    fs::write(tex_dir.join("example_icon.png"), tiny_png)?;
    fs::write(gui_tex.join("main_gui.png"), tiny_png)?;
    fs::write(item_tex.join("demo_item.png"), tiny_png)?;
    fs::write(item_tex.join("gui").join("next.png"), tiny_png)?;
    fs::write(ent_tex.join("demo_cow.png"), tiny_png)?;

    let mut readme = root.join("README.md");
    let _ = &mut readme;
    fs::write(
        root.join("README.md"),
        format!(
            "# {}\n\nMinecraft resource pack Project authored with PackCreator.\n\n## Layout\n\n- `build.pk` — mappings (WHOLE) + pack config\n- `src/main/configuration/` — pack YAML sections\n- `src/main/resourcepack/` — static assets\n\n## Build\n\n```\npack-creator build .\n```\n\nOutputs:\n- `{}` — project pack tree (`pack.yml` + configuration + resourcepack)\n- `{}` — client resource pack zip\n",
            build.project.name, build.export.pack_dir, build.export.resource_pack_zip
        ),
    )?;

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct PackYml {
    pub enable: bool,
    pub namespace: String,
    pub author: String,
    pub version: String,
    pub description: String,
}

impl From<&BuildPk> for PackYml {
    fn from(b: &BuildPk) -> Self {
        Self {
            enable: b.project.enable,
            namespace: b.project.namespace.clone(),
            author: b.project.author.clone(),
            version: b.project.version.clone(),
            description: b.project.description.clone(),
        }
    }
}
