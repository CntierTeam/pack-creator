---
name: pack-creator
description: >-
  Author and pack Minecraft resource packs with the PackCreator Rust TUI/CLI.
  Covers Project layout (build.pk + src/main), WHOLE mappings, multi-version
  export.variants (26.2 / pack_format 88), zip compression, override{} client
  translation-key replacements (with all{} fill), Component baking for lang
  values (<image>/<shift>), pack tree export, and resource_pack.zip generation.
  Trigger on: PackCreator, pack-creator, build.pk, Minecraft resource pack,
  WHOLE mappings, override lang, translation key, 26.2, pack_format.
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
3. **All pack config aggregates in `build.pk`** (items/images/gui/entities/`override`/…). YAML under `configuration/` is optional overlay; `build.pk` wins.
4. `mappings.mode = WHOLE` embeds the full `block_state_mappings` table on pack export.
5. Use `export.variants` for multi-version zips (`26_2` → pack_format 88); `zip { level = N }` for compression; `pack.mcmeta` always includes min/max/supported_formats.
6. Client language refs: put translation-key overrides in **`override { }`** (any `xxx.xxx`). Use **`all { }`** as defaults; `zh_cn` / `en_us` only need differing keys. Locale-specific wins; `all` fills holes. Output: `assets/minecraft/lang/<locale>.json`.
7. Lang/override **values** may use `<image:ns:id>`, `<shift:N>`, colors — baked to PUA + `§` at pack time (client lang is plain string).
8. On fuseblk mounts (e.g. `/projectsDir`), keep Cargo artifacts on a native FS (`CARGO_TARGET_DIR`).
9. Code/comments in English; user-facing replies follow the user’s language.

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
| Build pack + zip | `pack-creator build [dir] [--variant NAME]…` |

## Project layout

```text
MyPack/
  build.pk             # ALL configuration (meta + content + override)
  src/main/
    pack.yml
    configuration/     # optional YAML overlays
    resourcepack/      # static assets (+ optional overlays/<variant>/)
  build/               # created by build
    pack/<name>/
    resource_pack.zip
    cache/
    report.json
```

Read [references/project-layout.md](references/project-layout.md) for `build.pk` DSL and section keys.

## `override { }` (client translation keys)

```text
override {
  all {
    "block.minecraft.dirt" = "Soft Dirt"
    "gui.done" = "Done!"
  }
  zh_cn {
    "item.minecraft.apple" = "脆甜苹果"
    "gui.done" = "完成！"      // wins over all
  }
  en_us {
    "item.minecraft.apple" = "Crispy Apple"
    // dirt / gui.done filled from all
  }
}
```

- Any Minecraft translation key works (`item.*`, `entity.*`, `death.*`, `enchantment.*`, …).
- Shorthands: `item_name:ns:id` → `item.ns.id` (also `block_name:`, `entity:`, …).
- `override` wins over `lang` on the same key.
- Item `data.display-name` also auto-fills `item.ns.id` when not set explicitly.

## Typical agent workflow

1. Confirm cwd is PackCreator repo or an existing Project (`build.pk` present).
2. Create: `pack-creator new ./MyPack --name MyPack --namespace mypack`.
3. Edit `build.pk` (aggregate config, especially `override` / `items` / `images`) + assets under `resourcepack/`.
4. `pack-creator check .` then `pack-creator build .` (optional `--variant 26_2`).
5. Use `build/pack/<name>/` as the project pack tree; ship `build/resource_pack.zip` (or per-variant zips) to clients.

## Install this skill into Codex

```bash
./scripts/install-codex-skill.sh              # copy
./scripts/install-codex-skill.sh link         # symlink
./scripts/install-codex-skill.sh release      # pull from GitHub
curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install-codex-skill.sh | bash -s -- release
```

## References

- Layout & DSL: [references/project-layout.md](references/project-layout.md)
