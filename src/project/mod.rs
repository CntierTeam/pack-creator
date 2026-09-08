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

    // Optional YAML overrides only — all real config lives in build.pk
    fs::write(
        Project::configuration_dir(root).join("README.txt"),
        "Optional YAML overlays. Primary configuration is aggregated in build.pk.\n\
         If the same id exists in both places, build.pk wins.\n",
    )?;

    let pack_yml = format!(
        "enable: {}\nnamespace: {}\nauthor: {}\nversion: {}\ndescription: {}\n",
        build.project.enable,
        build.project.namespace,
        build.project.author,
        build.project.version,
        build.project.description
    );
    fs::write(Project::src_main(root).join("pack.yml"), pack_yml)?;

    let tex_dir = Project::resourcepack_dir(root)
        .join("assets")
        .join(ns)
        .join("textures")
        .join("font");
    fs::create_dir_all(&tex_dir)?;

    let gui_tex = Project::resourcepack_dir(root)
        .join("assets")
        .join(ns)
        .join("textures")
        .join("font")
        .join("gui");
    fs::create_dir_all(&gui_tex)?;

    let item_tex = Project::resourcepack_dir(root)
        .join("assets")
        .join(ns)
        .join("textures")
        .join("item");
    fs::create_dir_all(item_tex.join("gui"))?;

    let ent_tex = Project::resourcepack_dir(root)
        .join("assets")
        .join(ns)
        .join("textures")
        .join("entity");
    fs::create_dir_all(&ent_tex)?;

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

    fs::write(
        root.join("README.md"),
        format!(
            "# {}\n\nMinecraft resource pack Project authored with PackCreator.\n\n## Layout\n\n- `build.pk` — **all** pack configuration (mappings + items/images/gui/entities/…)\n- `src/main/resourcepack/` — static textures / sounds / assets\n- `src/main/configuration/` — optional YAML overlays (build.pk wins on clash)\n\n## Build\n\n```\npack-creator build .\n```\n\nOutputs:\n- `{}` — project pack tree (`pack.yml` + configuration + resourcepack)\n- `{}` — client resource pack zip\n",
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
