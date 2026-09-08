# Project layout & build.pk

## Required roots

| Path | Role |
|------|------|
| `build.pk` | Gradle-inspired DSL: project meta, mappings, pack features, export paths |
| `src/main/configuration/` | Craft-Engine YAML sections (recursive) |
| `src/main/resourcepack/` | Static Minecraft assets merged into zip / CE resourcepack |
| `src/main/pack.yml` | Author-facing meta (export also writes CE `pack.yml`) |

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
  ceResources = "build/ce-resources"
  resourcePackZip = "build/resource_pack.zip"
}
```

- `WHOLE`: on build, emit full CE `block_state_mappings.yml` unless the Project already defines that section.
- `CUSTOM`: skip embedding the whole table.

## Configuration section aliases (scan)

Canonical keys (aliases accepted):

`templates`, `global-variables`, `images`, `emojis`, `lang`, `translations`, `sounds`, `jukebox-songs`, `equipments`, `items`, `blocks`, `block_state_mappings`, `furniture`, `paintings`, `recipes`, `categories`, `loot-tables`, `config_factory`

CE-style suffixes like `lang#items` are treated as `lang`.

## Build outputs

| Output | Use |
|--------|-----|
| `build/ce-resources/<name>/` | Drop into `plugins/CraftEngine/resources/` |
| `build/resource_pack.zip` | Client resource pack (fonts/lang/sounds/equipment + static) |
| `build/cache/font/*.json` | Stable font codepoint allocator cache |
| `build/report.json` | Machine-readable build summary |

## What local zip does / does not

**Does:** merge `resourcepack/`, generate font providers from `images`, offset chars when enabled, client `lang`, `sounds.json`, equipment JSON, `pack.mcmeta`.

**Does not fully replicate CE:** legacy/modern item model overrides, blockstates visual allocation, obfuscation, overlays. Those stay as configuration for Craft-Engine runtime packing.
