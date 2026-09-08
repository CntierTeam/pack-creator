# Project layout & build.pk

## Required roots

| Path | Role |
|------|------|
| `build.pk` | **All** pack configuration: project meta, mappings, features, export, **and** content sections (`items` / `images` / `entityModels` / …) |
| `src/main/resourcepack/` | Static Minecraft assets merged into zip / pack tree |
| `src/main/configuration/` | Optional YAML overlays only (`build.pk` wins on id clash) |
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
    gui = true
    entities = true
  }
}

export {
  packDir = "build/pack"
  resourcePackZip = "build/resource_pack.zip"
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

entityModels {
  "mypack:demo_cow" {
    model {
      path = "mypack:entity/demo_cow"
      parent = "minecraft:block/block"
      textures { all = "mypack:entity/demo_cow" }
    }
  }
}
```

Content roots (aliases accepted): `images`, `emojis`, `items`, `blocks`, `entityModels`, `lang`, `sounds`, `equipments`, `furniture`, `paintings`, `templates`, `globalVariables`, `recipes`, `categories`, `lootTables`, `gui` (flattens nested `images`/`items`).

- `WHOLE`: on build, emit full `block_state_mappings.yml` unless the Project already defines that section.
- `CUSTOM`: skip embedding the whole table.
- Quoted keys (`"ns:id"`) and lists (`keywords = [":)", ":hi:"]`) are supported.
- `export.variants` → one ZIP per Minecraft line (`26_2` = pack_format **88** for 26.2). Keys may use `26.2`; normalized to `26_2`.
- `zip { method = DEFLATED; level = 6 }` controls native ZIP compression (0..=9).
- Optional assets: `src/main/resourcepack/overlays/<variant>/` merged into that variant’s zip.

## Configuration section aliases (YAML overlays)

Canonical keys (aliases accepted):

`templates`, `global-variables`, `images`, `emojis`, `lang`, `translations`, `sounds`, `jukebox-songs`, `equipments`, `items`, `blocks`, `block_state_mappings`, `furniture`, `paintings`, `recipes`, `categories`, `loot-tables`, `config_factory`, `entity_models`

## Build outputs

| Output | Use |
|--------|------|
| `build/pack/<name>/` | Pack tree: `pack.yml` + `configuration/` (emitted from build.pk) + `resourcepack/` |
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
