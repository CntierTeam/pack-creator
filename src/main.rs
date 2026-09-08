use anyhow::Context;
use clap::{Parser, Subcommand};
use pack_creator::app::App;
use pack_creator::{build_project, Project};
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
    /// Build pack tree + resource_pack.zip
    Build {
        /// Project root (contains build.pk)
        #[arg(default_value = ".")]
        path: PathBuf,
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
        Commands::Build { path } => {
            let project = Project::open(&path)
                .with_context(|| format!("open project at {}", path.display()))?;
            let report = build_project(&project).context("build failed")?;
            println!("pack dir: {}", report.pack_dir.display());
            println!("ZIP:      {}", report.resource_pack_zip.display());
            println!(
                "sections={} fonts={} langs={} sounds={} copied={}",
                report.sections.len(),
                report.fonts_written,
                report.langs_written,
                report.sounds_written,
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
            println!("config files: {}", idx.files.len());
            for (section, hits) in &idx.sections {
                println!("  - {section}: {} hit(s)", hits.len());
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
