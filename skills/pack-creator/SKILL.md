---
name: pack-creator
description: >-
  Operate PackCreator CLI/TUI `pack-creator` by running it for the user to
  author and export Minecraft resource packs. Covers Project layout (build.pk +
  src/main), WHOLE mappings, multi-version export.variants (26.2 / pack_format
  88), zip compression, override{} client translation-key replacements (with
  all{} fill), Component baking for lang values (<image>/<shift>), pack tree
  export, and resource_pack.zip generation. Prefer shell execution over pasting
  recipes. Trigger on: PackCreator, pack-creator, build.pk, Minecraft resource
  pack, WHOLE mappings, override lang, translation key, 26.2, pack_format.
license: GPL-3.0
metadata:
  short-description: 代跑 pack-creator（建包/校验/导出）
---

# pack-creator

产品：**`pack-creator`** — Minecraft 资源包 Project 的 CLI/TUI（`build.pk` + zip 导出）。

你是 **操作员**：用户要新建包、改 `build.pk`、校验、按变体导出 → **自己在 shell 执行 `pack-creator`**，不要只拼命令或教用户手搓 zip。

本 skill 是 **execute-first**：代跑产品，不是开发 PackCreator crates。DSL/布局细节见 [references/project-layout.md](references/project-layout.md)。

Repo: https://github.com/CntierTeam/pack-creator

## Agent 硬规则

1. **执行优先**：能跑就跑。二进制：`pack-creator` 或 `~/.local/bin/pack-creator`；没有就先装。
2. **禁止**用「组装指令 / SAMPLE / YOUR_CLI / 长篇手工 zip 教程」代替执行。短句说明 → 立刻跑 → 根据输出继续。
3. 用户要建包/构建/导出 → **马上** `pack-creator new` / `check` / `build`。缺目录名、namespace、variant 时只问缺的那一项，问完继续跑。
4. 命令名永远 **`pack-creator`**，禁止 `SAMPLE` / `YOUR_CLI`。
5. Project **必须**有 `build.pk` + `src/main/`（`configuration/` + `resourcepack/`）。**配置全在 `build.pk`**；YAML overlay 可选且 **`build.pk` 胜出**。
6. 保留产品语义：`mappings.mode = WHOLE`；`export.variants`（`26_2` → pack_format **88**）；`zip { level = N }`；客户端翻译键放 **`override { }`**（`all { }` 填默认，locale 覆盖）；值可含 `<image:…>` / `<shift:N>`，构建时烘焙。
7. TUI：用户明确要交互全屏时再 `pack-creator` / `pack-creator tui`；Agent 代控优先子命令。

## 标准代跑流

```bash
command -v pack-creator || ~/.local/bin/pack-creator --help

# 无 Project 则创建：
pack-creator new ./MyPack --name MyPack --namespace mypack

# 已有 Project：
cd ./MyPack   # 或传 [dir]
pack-creator check .
pack-creator build .                    # 可选：--variant 26_2
# 交付：build/pack/<name>/ 与 build/resource_pack.zip（或 per-variant zip）
```

## 意图 → 怎么跑

| 用户意图 | 执行 |
|----------|------|
| 新建资源包 Project | `pack-creator new <dir> --name <Name> --namespace <ns>` |
| 校验 | `pack-creator check [dir]` |
| 构建 / 导出 zip | `pack-creator build [dir] [--variant NAME]…` |
| 多版本（26.2 / format 88） | 在 `build.pk` 配 `export.variants`，再 `build --variant 26_2` |
| 改翻译键 / override | 编辑 `build.pk` 的 `override { }`，再 `check` → `build` |
| WHOLE mappings | `build.pk` 里 `mappings { mode = WHOLE }`，再 build |
| 要 TUI | 启动 `pack-creator` / `pack-creator tui` |

## Command map

| Need | Command |
|------|---------|
| TUI | `pack-creator` / `pack-creator tui` |
| New Project | `pack-creator new <dir> --name <Name> --namespace <ns>` |
| Validate | `pack-creator check [dir]` |
| Build pack + zip | `pack-creator build [dir] [--variant NAME]…` |

## Project layout（代跑时要认）

```text
MyPack/
  build.pk             # ALL configuration（含 override / items / images / …）
  src/main/
    pack.yml
    configuration/     # optional YAML overlays
    resourcepack/      # static assets（+ overlays/<variant>/）
  build/               # created by build
    pack/<name>/
    resource_pack.zip
    cache/
    report.json
```

### `override { }`（客户端翻译键）

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
  }
}
```

- 任意 MC 翻译键；简写 `item_name:ns:id` → `item.ns.id`。
- locale 胜 `all`；`override` 胜 `lang`。输出：`assets/minecraft/lang/<locale>.json`。

## Install（仅当本机没有 pack-creator）

```bash
curl -fsSL https://raw.githubusercontent.com/CntierTeam/pack-creator/main/scripts/install.sh | bash
command -v pack-creator && pack-creator --help
```

https://github.com/CntierTeam/pack-creator
