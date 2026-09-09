---
name: pack-creator
description: >-
  Author and pack Minecraft resource packs with the PackCreator CLI/TUI.
  Covers Project layout (build.pk + src/main), WHOLE mappings, multi-version
  export.variants (26.2 / pack_format 88), zip compression, override{} client
  translation-key replacements (with all{} fill), Component baking for lang
  values (<image>/<shift>), pack tree export, and resource_pack.zip generation.
  Trigger on: PackCreator, pack-creator, build.pk, Minecraft resource pack,
  WHOLE mappings, override lang, translation key, 26.2, pack_format.
license: GPL-3.0
metadata:
  short-description: Use pack-creator to author Minecraft resource packs
---

# pack-creator

Use the **`pack-creator` binary** to author PackCreator Projects and export a
pack tree + client `resource_pack.zip`. This skill is for **using** the tool on
pack projects — not for developing the PackCreator source repo.

Binary install: https://github.com/CntierTeam/pack-creator

## Hard rules

1. Prefer the **installed `pack-creator` binary** over inventing custom pack scripts or hand-writing zip trees.
2. A Project **must** contain `build.pk` and `src/main/` (`configuration/` + `resourcepack/`).
3. **All pack config lives in `build.pk`** (items / images / gui / entities / `override` / …). YAML under `configuration/` is optional overlay; **`build.pk` wins**.
4. `mappings.mode = WHOLE` embeds the full `block_state_mappings` table on pack export.
5. Multi-version: `export.variants` (`26_2` → pack_format **88**); `zip { level = N }`; `pack.mcmeta` gets min/max/supported_formats.
6. Client translation keys: put overrides in **`override { }`**. Use **`all { }`** for shared defaults; locale blocks only need diffs. Locale wins over `all`; `override` wins over `lang`. Output: `assets/minecraft/lang/<locale>.json`.
7. Lang/override **values** may use `<image:ns:id>`, `<shift:N>`, colors — baked to PUA + `§` at pack time.
8. User-facing replies follow the user’s language.

## Install / resolve the binary

```bash
# Preferred: Release install onto PATH
curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install.sh | bash
command -v pack-creator
pack-creator --help
```

## Command map

| Need | Command |
|------|---------|
| TUI | `pack-creator` / `pack-creator tui` |
| New Project | `pack-creator new <dir> --name <Name> --namespace <ns>` |
| Validate | `pack-creator check [dir]` |
| Build pack + zip | `pack-creator build [dir] [--variant NAME]…` |

## Project layout (the pack you edit)

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

DSL details: [references/project-layout.md](references/project-layout.md).

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
- Item `data.display-name` also auto-fills `item.ns.id` when not set explicitly.

## Typical agent workflow

1. Confirm the cwd (or target dir) is a **PackCreator Project** (`build.pk` present), or create one.
2. Create if needed: `pack-creator new ./MyPack --name MyPack --namespace mypack`.
3. Edit `build.pk` (`override` / `items` / `images` / …) and assets under `src/main/resourcepack/`.
4. `pack-creator check .` then `pack-creator build .` (optional `--variant 26_2`).
5. Deliver `build/pack/<name>/` and/or `build/resource_pack.zip` (or per-variant zips).

## References

- Layout & DSL: [references/project-layout.md](references/project-layout.md)
