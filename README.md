# PackCreator

Rust TUI / CLI for authoring **Craft-Engine** resource packs.

Repository: [CntierTeam/pack-creator](https://github.com/CntierTeam/pack-creator)

Create a **Project** (`build.pk` + `src/main`), edit CE-compatible configuration, then build:

1. **CE resources** → drop into `plugins/CraftEngine/resources/<name>/`
2. **resource_pack.zip** → client pack (fonts / lang / sounds / equipment + static assets)

## Requirements

- Rust stable (`rustc` / `cargo`) for building from source
- Linux / Windows x86_64 release binaries via `scripts/install.sh`

If the source tree lives on a **fuseblk** mount (e.g. `/projectsDir`), put build artifacts on a native filesystem:

```bash
export CARGO_TARGET_DIR=/tmp/packcreator-target
```

## Install (binary from GitHub Releases)

```bash
curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install.sh | bash
# or pin a tag:
# curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install.sh | bash -s -- v0.1.0
```

Installs into `~/.local/bin/pack-creator` by default (`INSTALL_DIR` / `REPO` / `VERSION` / `ASSET` overridable).

## Build

```bash
cargo build
cargo build --release
cargo test
```

Binary name: `pack-creator`.

## Quick start

```bash
pack-creator new ./MyPack --name MyPack --namespace mypack
cd MyPack
# edit build.pk and src/main/configuration/*.yml
# put textures under src/main/resourcepack/assets/...
pack-creator check .
pack-creator build .
```

Outputs:

- `build/ce-resources/MyPack/` — Craft-Engine pack root (`pack.yml`, `configuration/`, `resourcepack/`)
- `build/resource_pack.zip` — client zip
- `build/report.json` — build summary

Launch the TUI (default when no subcommand):

```bash
pack-creator
# or
pack-creator tui
```

## Project layout

```text
MyPack/
  build.pk                 # Gradle-inspired config + WHOLE mappings
  src/main/
    pack.yml
    configuration/         # images, emoji, lang, sounds, items, blocks, ...
    resourcepack/          # static Minecraft assets
```

### build.pk (sketch)

```text
project {
  name = "MyPack"
  namespace = "mypack"
  version = "1.0.0"
}

mappings {
  mode = WHOLE   // embed full Craft-Engine block_state_mappings on CE export
  font {
    codepointStartingValue = 19968
    override("minecraft:default", 57344)
  }
  customModelData {
    startingValue = 10000
  }
}

pack {
  supportedVersion { min = "1.20.1" max = "1.21.4" }
  features { images = true /* ... */ }
}

export {
  ceResources = "build/ce-resources"
  resourcePackZip = "build/resource_pack.zip"
}
```

`WHOLE` copies Craft-Engine’s full `block_state_mappings` into the CE export (unless you already define that section).

## What gets packed locally vs by Craft-Engine

| Local zip | CE plugin runtime |
|-----------|-------------------|
| Font providers from `images` (+ offset chars) | Full item/block model generation |
| Client `lang` | CMD / visual block state allocation |
| `sounds.json`, equipment JSON | Overlays, obfuscation, conflict merge |
| Static `resourcepack/` merge | Hosting / delivery |

Items, blocks, furniture, recipes, etc. are authored as CE YAML and exported for the plugin.

## CLI

| Command | Meaning |
|---------|---------|
| `pack-creator` / `tui` | Interactive TUI |
| `new <path> --name … --namespace …` | Scaffold Project |
| `check [path]` | Scan configuration sections |
| `build [path]` | Export CE resources + zip |

## Releases / CI

- **CI** (`.github/workflows/ci.yml`): build + test on Linux gnu/musl and Windows.
- **Release** (`.github/workflows/release.yml`): push a `v*` tag (or `workflow_dispatch` with tag) → GitHub Release assets:
  - `pack-creator-linux-x86_64-musl` (+ alias `pack-creator-linux-x86_64`)
  - `pack-creator-linux-x86_64-gnu`
  - `pack-creator-windows-x86_64.exe`
  - matching `.sha256` files

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Codex skill

```bash
# from a checkout:
./scripts/install-codex-skill.sh              # copy
./scripts/install-codex-skill.sh link         # symlink
./scripts/install-codex-skill.sh release      # pull from GitHub main/tag

# without a checkout:
curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install-codex-skill.sh | bash -s -- release
```

## Tests

```bash
cargo test
```

## License

GPL-3.0
