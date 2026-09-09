# PackCreator

Rust TUI / CLI for authoring **Minecraft resource packs**.

Repository: [CntierTeam/pack-creator](https://github.com/CntierTeam/pack-creator)

Create a **Project** (`build.pk` + `src/main`), put **all configuration in `build.pk`**, drop textures under `resourcepack/`, then build:

1. **Pack tree** → `pack.yml` + `configuration/` + `resourcepack/`
2. **resource_pack.zip** → client resource pack (fonts / lang / sounds / equipment / item & entity models + static assets)

`src/main/configuration/*.yml` is optional overlay only; **`build.pk` wins** on the same id.

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
# edit build.pk — items / images / lang / entities / … all live here
# put textures under src/main/resourcepack/assets/...
pack-creator check .
pack-creator build .
```

Outputs:

- `build/pack/MyPack/` — pack tree (`pack.yml`, `configuration/`, `resourcepack/`)
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
  build.pk                 # ALL config: meta + mappings + content sections
  src/main/
    pack.yml
    configuration/         # optional YAML overlays (build.pk wins)
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
  mode = WHOLE   // embed full block_state_mappings on pack export
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
  packFormat = 34
  // modern mcmeta fields (derived from packFormat if omitted)
  minFormat = [34, 0]
  maxFormat = [34, 0]
  supportedFormats {
    minInclusive = 34
    maxInclusive = 34
  }
  features { images = true /* ... */ }
}

export {
  packDir = "build/pack"
  resourcePackZip = "build/resource_pack.zip"
  // Optional Asteri-style multi-version outputs:
  // variants {
  //   "26_1" { packFormat = 84; resourcePackZip = "build/resource_pack_26_1.zip" }
  //   "26_2" { packFormat = 88; resourcePackZip = "build/resource_pack_26_2.zip" }
  // }
  // defaultVariants = ["26_1", "26_2"]
}

zip {
  method = DEFLATED   // or STORED
  level = 6           // 0..=9
}

items {
  "mypack:demo_item" {
    material = "PAPER"
    model {
      path = "mypack:item/demo_item"
      generation {
        parent = "minecraft:item/generated"
        textures { layer0 = "mypack:item/demo_item" }
      }
    }
  }
}

images {
  "mypack:main_gui" {
    height = 140
    ascent = 18
    font = "minecraft:gui"
    file = "mypack:font/gui/main_gui.png"
  }
}

// Client translation-key overrides (any xxx.xxx the resource pack can replace)
// `all` fills keys that zh_cn / en_us omit; locale-specific entries win.
override {
  all {
    "block.minecraft.dirt" = "Soft Dirt"
    "gui.done" = "Done!"
  }
  zh_cn {
    "item.minecraft.apple" = "脆甜苹果"
    "gui.done" = "完成！"
  }
  en_us {
    "item.minecraft.apple" = "Crispy Apple"
  }
}
```

Content roots (aliases accepted): `items`, `images`, `emojis`, `entityModels`, `lang`, `override`, `sounds`, `equipments`, `blocks`, `furniture`, `paintings`, `templates`, `globalVariables`, `recipes`, `categories`, `lootTables`, `gui`.

Quoted keys (`"ns:id"`) and lists (`keywords = [":)", ":hi:"]`) are supported.

`WHOLE` embeds the full `block_state_mappings` table into the pack export (unless you already define that section).

### Multi-version export (26.2 etc.)

Define `export.variants` so each Minecraft line gets its own ZIP + `pack.mcmeta` formats (same idea as AsteriResourcePack):

| Variant key | Typical `packFormat` | Notes |
|-------------|----------------------|-------|
| `1_21_8` | 64 | |
| `26_1` | 84 | |
| `26_2` | **88** | Minecraft **26.2** |

`26.2` in DSL/`--variant` is normalized to `26_2`. Optional per-variant assets: `src/main/resourcepack/overlays/<name>/` (merged into `assets/`, not shipped as runtime overlays).

```bash
pack-creator build . --variant 26_2
```

### ZIP compression

```text
zip {
  method = DEFLATED  // or STORED
  level = 6          // 0..=9, default 6
}
```

## What the local zip includes

| Included now | Notes |
|--------------|-------|
| Font providers from `images` (+ offset chars) | GUI uses `font: minecraft:gui` |
| Client `lang` | Pack-local strings (`pack.x`, `item.ns.id`, …) |
| Client `override` | **Translation-key overrides**: any `xxx.xxx` ref (`item.minecraft.apple`, `gui.done`, `death.*`, …) → `assets/minecraft/lang/<locale>.json`. Wins over `lang` on clash. Values may use `<image>`/`<shift>`/colors. |
| `sounds.json`, equipment JSON | |
| **Item models** | `models/*.json`, `items/*.json`, legacy CMD overrides on `models/item/<material>.json` |
| **Entity models / texture replace** | `models/entity/*.json` + `replace_textures` copies |
| Static `resourcepack/` merge | |

Furniture client visuals reuse **item models** (ItemDisplay is runtime, not zip).

## CLI

| Command | Meaning |
|---------|---------|
| `pack-creator` / `tui` | Interactive TUI |
| `new <path> --name … --namespace …` | Scaffold Project |
| `check [path]` | List `build.pk` content sections, variants, zip settings |
| `build [path] [--variant NAME]…` | Export pack tree + one or more zips |

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
