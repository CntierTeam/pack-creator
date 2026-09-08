---
name: pack-creator
description: >-
  Author and pack Minecraft resource packs with the PackCreator Rust TUI/CLI.
  Covers Project layout (build.pk + src/main), WHOLE mappings, pack tree export,
  resource_pack.zip generation, and configuration sections (images/font, emoji,
  lang, sounds, equipment, items, blocks, furniture, templates, placeholders).
  Trigger on: PackCreator, pack-creator, build.pk, Minecraft resource pack,
  WHOLE mappings, block_state_mappings, font pack, placeholder pack.
license: GPL-3.0
metadata:
  short-description: Minecraft resource pack TUI/CLI (build.pk)
---

# pack-creator

Rust TUI/CLI for authoring **PackCreator Projects** that export a pack tree and
a client `resource_pack.zip`.

Repo: https://github.com/CntierTeam/pack-creator

## Hard rules

1. A Project **must** contain `build.pk` and `src/main/` (with `configuration/` + `resourcepack/`).
2. Prefer the **pack-creator binary** over re-implementing packing in ad-hoc scripts.
3. **All pack config aggregates in `build.pk`** (items/images/gui/entities/…). YAML under `configuration/` is optional overlay; `build.pk` wins.
4. `mappings.mode = WHOLE` embeds the full `block_state_mappings` table on pack export.
5. Local zip generates fonts (incl. GUI), lang, sounds, equipment, **item models**, **entity model/texture replace**, and static assets.
6. On fuseblk mounts (e.g. `/projectsDir`), keep Cargo artifacts on a native FS (`CARGO_TARGET_DIR`).
7. Code/comments in English; user-facing replies follow the user’s language.

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
export CARGO_TARGET_DIR=/tmp/packcreator-target   # if source is on fuseblk
cargo build
cargo build --release
cargo test
```

Install from Releases:

```bash
curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install.sh | bash
```

## Command map

| Need | Command |
|------|---------|
| TUI | `pack-creator` / `pack-creator tui` |
| New Project | `pack-creator new <dir> --name <Name> --namespace <ns>` |
| Validate | `pack-creator check [dir]` |
| Build pack + zip | `pack-creator build [dir]` |

## Project layout

```text
MyPack/
  build.pk             # ALL configuration (meta + content sections)
  src/main/
    pack.yml
    configuration/     # optional YAML overlays
    resourcepack/      # static assets
  build/               # created by build
    pack/<name>/
    resource_pack.zip
    cache/
    report.json
```

Read [references/project-layout.md](references/project-layout.md) for `build.pk` DSL and section keys.

## Typical agent workflow

1. Confirm cwd is PackCreator repo or an existing Project (`build.pk` present).
2. Create: `pack-creator new ./MyPack --name MyPack --namespace mypack`.
3. Edit `build.pk` (aggregate config) + assets under `resourcepack/`.
4. `pack-creator check .` then `pack-creator build .`.
5. Use `build/pack/<name>/` as the project pack tree; ship `build/resource_pack.zip` to clients.

## Install this skill into Codex

```bash
./scripts/install-codex-skill.sh              # copy
./scripts/install-codex-skill.sh link         # symlink
./scripts/install-codex-skill.sh release      # pull from GitHub
curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install-codex-skill.sh | bash -s -- release
```

## References

- Layout & DSL: [references/project-layout.md](references/project-layout.md)
