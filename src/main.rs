use anyhow::Context;
use clap::{Parser, Subcommand};
use pack_creator::app::App;
use pack_creator::{build_project_filtered, Project};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "pack-creator", version, about = "Minecraft resource pack TUI / CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch interactive TUI
    Tui,
    /// Create a new Project (build.pk + src/main)
    New {
        /// Project directory
        path: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        namespace: String,
    },
    /// Build pack tree + resource_pack.zip (one or more variants)
    Build {
        /// Project root (contains build.pk)
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Build only these export.variants (repeatable). Accepts `26.2` or `26_2`.
        #[arg(long = "variant", value_name = "NAME")]
        variants: Vec<String>,
    },
    /// Validate project layout and scan configuration sections
    Check {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Tui) {
        Commands::Tui => {
            let mut app = App::new();
            app.run().map_err(|e| anyhow::anyhow!(e))?;
        }
        Commands::New {
            path,
            name,
            namespace,
        } => {
            let project = Project::create(&path, &name, &namespace)
                .with_context(|| format!("create project at {}", path.display()))?;
            println!(
                "created {} (namespace={})",
                project.root.display(),
                project.build.project.namespace
            );
        }
        Commands::Build { path, variants } => {
            let project = Project::open(&path)
                .with_context(|| format!("open project at {}", path.display()))?;
            let report = build_project_filtered(&project, &variants).context("build failed")?;
            println!("pack dir: {}", report.pack_dir.display());
            if report.resource_pack_zips.len() <= 1 {
                println!("ZIP:      {}", report.resource_pack_zip.display());
            } else {
                println!("ZIPs:");
                for (name, zip) in report.variants_built.iter().zip(&report.resource_pack_zips) {
                    println!("  - {name}: {}", zip.display());
                }
            }
            println!(
                "sections={} fonts={} langs={} sounds={} items={} entities={} copied={}",
                report.sections.len(),
                report.fonts_written,
                report.langs_written,
                report.sounds_written,
                report.item_models,
                report.entity_models + report.entity_texture_replacements,
                report.files_copied
            );
        }
        Commands::Check { path } => {
            let project = Project::open(&path)
                .with_context(|| format!("open project at {}", path.display()))?;
            let idx = pack_creator::config::scan_configuration(&Project::configuration_dir(
                &project.root,
            ))?;
            println!("project: {}", project.build.project.name);
            println!("mappings: {:?}", project.build.mappings.mode);
            println!(
                "pack_format: {} (min={:?} max={:?})",
                project.build.pack.pack_format,
                project.build.pack.min_format,
                project.build.pack.max_format
            );
            println!(
                "zip: method={:?} level={}",
                project.build.zip.method,
                project.build.zip.level
            );
            if project.build.export.variants.is_empty() {
                println!(
                    "export: single → {}",
                    project.build.export.resource_pack_zip
                );
            } else {
                println!("export variants:");
                for (name, v) in &project.build.export.variants {
                    println!(
                        "  - {name}: format={} → {}",
                        v.pack_format, v.resource_pack_zip
                    );
                }
                if !project.build.export.default_variants.is_empty() {
                    println!(
                        "defaultVariants: {}",
                        project.build.export.default_variants.join(", ")
                    );
                }
            }
            println!("build.pk content sections: {}", project.build.contents.len());
            for (section, entries) in &project.build.contents {
                println!("  - {section}: {} entr(y/ies)", entries.len());
            }
            println!("yaml overlay files: {}", idx.files.len());
            for (section, hits) in &idx.sections {
                println!("  - overlay {section}: {} hit(s)", hits.len());
            }
            if !idx.unknown_roots.is_empty() {
                println!(
                    "unknown roots: {}",
                    idx.unknown_roots.iter().cloned().collect::<Vec<_>>().join(", ")
                );
            }
        }
    }
    Ok(())
}
