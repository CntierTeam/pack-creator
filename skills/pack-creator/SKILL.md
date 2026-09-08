---
name: pack-creator
description: >-
  Author and pack Craft-Engine resource packs with the PackCreator Rust TUI/CLI.
  Covers Project layout (build.pk + src/main), WHOLE mappings, CE resources export,
  resource_pack.zip generation, and configuration sections (images/font, emoji,
  lang, sounds, equipment, items, blocks, furniture, templates, placeholders).
  Trigger on: PackCreator, pack-creator, build.pk, Craft-Engine pack, CE resource
  pack, WHOLE mappings, block_state_mappings, Minecraft resource pack authoring.
license: GPL-3.0
metadata:
  short-description: Craft-Engine pack authoring TUI/CLI (build.pk)
---

# pack-creator

Rust TUI/CLI for authoring **PackCreator Projects** that export into Craft-Engine
`resources/<pack>/` layout and optionally a client `resource_pack.zip`.

Repo root: this skill ships inside the PackCreator repository.

## Hard rules

1. A Project **must** contain `build.pk` and `src/main/` (with `configuration/` + `resourcepack/`).
2. Prefer the **pack-creator binary** over re-implementing CE packing logic in ad-hoc scripts.
3. `mappings.mode = WHOLE` embeds Craft-Engine’s full `block_state_mappings` on CE export.
4. Local zip generates fonts/lang/sounds/equipment + static assets; **item/block model + CMD overrides** are forwarded as CE YAML for the plugin to assemble at runtime.
5. On fuseblk mounts (e.g. `/projectsDir`), keep Cargo artifacts on a native FS (`CARGO_TARGET_DIR` / `.cargo/config.toml`).
6. Code/comments in English; user-facing replies follow the user’s language.

## Resolve the binary

```bash
command -v pack-creator
# or
/tmp/packcreator-target/debug/pack-creator --help
./target/debug/pack-creator --help
./target/release/pack-creator --help
```

Build:

```bash
# if source is on fuseblk / NTFS:
export CARGO_TARGET_DIR=/tmp/packcreator-target
cargo build
cargo build --release
cargo test
```

## Command map

| Need | Command |
|------|---------|
| TUI | `pack-creator` / `pack-creator tui` |
| New Project | `pack-creator new <dir> --name <Name> --namespace <ns>` |
| Validate | `pack-creator check [dir]` |
| Build CE + zip | `pack-creator build [dir]` |

## Project layout

```text
MyPack/
  build.pk
  src/main/
    pack.yml
    configuration/     # CE YAML sections
    resourcepack/      # static assets
  build/               # created by build
    ce-resources/<name>/
    resource_pack.zip
    cache/
    report.json
```

Read [references/project-layout.md](references/project-layout.md) for `build.pk` DSL and section keys.

## Typical agent workflow

1. Confirm cwd is PackCreator repo or an existing Project (`build.pk` present).
2. Create: `pack-creator new ./MyPack --name MyPack --namespace mypack`.
3. Edit `build.pk` + `src/main/configuration/*.yml` + assets under `resourcepack/`.
4. `pack-creator check .` then `pack-creator build .`.
5. Hand CE output to `plugins/CraftEngine/resources/<name>/` (copy `build/ce-resources/<name>/`).
6. For client-only font/lang/sound packs, use `build/resource_pack.zip`.

## Install this skill into Codex

From the PackCreator repo root:

```bash
./scripts/install-codex-skill.sh              # copy → ~/.codex/skills/pack-creator
./scripts/install-codex-skill.sh link         # symlink for live edits
./scripts/install-codex-skill.sh release      # pull from GitHub (main or VERSION=v…)
```

Without a checkout:

```bash
curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install-codex-skill.sh | bash -s -- release
```

Override home: `CODEX_HOME=/path/to/codex ./scripts/install-codex-skill.sh`

## Install the binary from Releases

```bash
curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install.sh | bash
```

## References

- Layout & DSL: [references/project-layout.md](references/project-layout.md)
- CE pack assembly reference: sibling repo `craft-engine` (`AbstractPackManager`)
- Repo: https://github.com/CntierTeam/pack-creator
