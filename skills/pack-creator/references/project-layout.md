# Project layout & build.pk

## Required roots

| Path | Role |
|------|------|
| `build.pk` | Gradle-inspired DSL: project meta, mappings, pack features, export paths |
| `src/main/configuration/` | Pack YAML sections (recursive) |
| `src/main/resourcepack/` | Static Minecraft assets merged into zip / pack tree |
| `src/main/pack.yml` | Author-facing meta (export also writes `pack.yml`) |

## build.pk essentials

```text
project {
  name = "MyPack"
  namespace = "mypack"
  version = "1.0.0"
  enable = true
}

mappings {
  mode = WHOLE          // or CUSTOM
  font {
    codepointStartingValue = 19968
    offsetCharacters = true
    override("minecraft:default", 57344)
  }
  customModelData {
    startingValue = 10000
  }
}

pack {
  packFormat = 34
  supportedVersion { min = "1.20.1" max = "1.21.4" }
  features {
    images = true
    emoji = true
    lang = true
    sounds = true
    equipment = true
    paintings = true
    items = true
    blocks = true
    furniture = true
    templates = true
    placeholders = true
  }
}

export {
  packDir = "build/pack"
  resourcePackZip = "build/resource_pack.zip"
}
```

- `WHOLE`: on build, emit full `block_state_mappings.yml` unless the Project already defines that section.
- `CUSTOM`: skip embedding the whole table.

## Configuration section aliases (scan)

Canonical keys (aliases accepted):

`templates`, `global-variables`, `images`, `emojis`, `lang`, `translations`, `sounds`, `jukebox-songs`, `equipments`, `items`, `blocks`, `block_state_mappings`, `furniture`, `paintings`, `recipes`, `categories`, `loot-tables`, `config_factory`

Suffixes like `lang#items` are treated as `lang`.

## Build outputs

| Output | Use |
|--------|-----|
| `build/pack/<name>/` | Pack tree: `pack.yml` + `configuration/` + `resourcepack/` |
| `build/resource_pack.zip` | Client resource pack (fonts/lang/sounds/equipment + static) |
| `build/cache/font/*.json` | Stable font codepoint allocator cache |
| `build/report.json` | Machine-readable build summary |

## What local zip does / does not

**Does:**

- merge `resourcepack/`
- `images` → `assets/<ns>/font/<name>.json` (GUI: set `font: minecraft:gui`)
- item `generation` / `texture` → `assets/<ns>/models/...json`
- modern `assets/<ns>/items/<id>.json`
- legacy CMD overrides → `assets/minecraft/models/item/<material>.json`
- `entity_models.model` → `assets/<ns>/models/entity/...json`
- `entity_models.replace_textures` → copy PNG into pack (e.g. vanilla entity path)

**Does not (yet):** full blockstate visual packing, OptiFine CEM `.jem`, overlays, obfuscation.
