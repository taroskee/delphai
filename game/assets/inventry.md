# アセット在庫

> `game/assets/` に含まれるアセットの一覧と、Phase 1 での適用状況。

---

## 3D モデル（GLB）

### `animal/Animals_FREE.glb` (1.45 MB)

GLB ヘッダ解析で確認したトップレベルシーンルート（各動物は独立ノード）:

| ノード名 | 備考 |
|---|---|
| `tiger` | |
| `horse.001` | |
| `dog.001` | |
| **`deer`** | **Phase 1 で採用（鹿モデル）**。子に `deer_001`（Mesh+Skin）と `Root`（Skeleton）を持つ。`GlbLoader.load_subscene` で `deer` ごと切り出して使用。 |
| `kitty_001` | |
| `pinguin_001` | |
| `chicken_001` | |

- **animations: []（空）** — Idle/Run アニメはこのパックには含まれていない。`Sprint 10` の `AnimationPlayer` 項目は達成不可なのでスコープ外。
- 適用箇所: `game/scripts/animal_factory.gd::make_deer()` → `DEER_NODE_NAME = "deer"`, `DEER_SCALE = 0.45`
- フェード: `GlbLoader.prepare_fade_materials` が全ての `MeshInstance3D` 子孫を走査し、surface ごとに `StandardMaterial3D` へ複製 + `TRANSPARENCY_ALPHA` を付与。Tween は `set_parallel(true)` で複数マテリアルを同時フェード。

### `nature/simple_nature_pack_glb.glb` (283 KB)

トップレベル 16 ノード:

| ノード名 | Phase 1 適用 |
|---|---|
| `Bamboo_Tree_01` / `_02` / `_03` / `_01.001` | — |
| **`Oak_Tree_01`** | **`terrain_builder.gd::_add_tree` で T_FOREST タイルに使用**（`TREE_SCALE = 0.35`） |
| `Oak_Tree_02` | — |
| `Pine_Tree_01` / `_02` | — |
| **`Plant_01`** | **`resource_factory.gd::_make_berry_bush()` で berry_bush に使用**（`BERRY_SCALE = 0.25`、外側 `Node3D` ラッパで `world.gd` の `.scale = qty` と分離） |
| `Plant_02` / `_03` | — |
| **`Rock_01`** | **`resource_factory.gd::_make_water_source()` で water_source のリング配置に使用**（3個、半径 0.35、`WATER_ROCK_SCALE = 0.28`）。水メッシュが無いため「水辺を示す石のサークル」として代用。 |
| `Rock_02` / `_03` | — |
| **`Tree_Stump_01`** | **`citizen_factory.gd::_make_body()` で住民の仮置き胴体として使用**（`BODY_SCALE = 0.6`）。Unreal→Godot の人型モデル変換が完了するまでのプレースホルダ。 |
| `Tree_Stump_02` | — |

- animations: なし。
- 水源メッシュは含まれていない → `Rock_01` を 3 個リング状に配置して「水辺」を示す（Sprint 12.5 で primitive cylinder を撤去）。

### `geography/terrian.glb` (78 MB)

- 構造: Sketchfab 製モノリス (`Sketchfab_model` → `terrian.obj.cleaner.materialmerger.gles` → `Object_2..Object_6` の 5 メッシュ、マテリアル 2、テクスチャ 7 枚 / ~57 MB、animation なし)。
- **タイル分割されていない** → 格子マップ (`get_terrain(col,row)`) とは独立した「装飾背景」扱い。歩行可否・地形判定は既存 bitmap を維持。
- 適用箇所: `TerrainBuilder.build_visual_backdrop()` が衝突プレーン生成直後に `PackedScene` として instantiate し、マップ中心へ配置。
- チューニング定数: `GROUND_GLB_SCALE = 0.15`、`GROUND_GLB_Y = -0.05`。Sketchfab 出力は座標系が不定なので Godot 実機で視覚調整する前提。

### `animal/Separate_assets_glb.zip`

個別動物の GLB（展開済みの可能性あり）。未調査。Phase 2 以降で個別アニメーションが必要になった時に再評価。

---

## 住民（人型）GLB

**未取得**。所有している人型モデルは Unreal Engine 向けで Godot への変換が未完了。Sprint 12.5 で `CitizenFactory` は `simple_nature_pack_glb.glb → Tree_Stump_01` を人型プレースホルダとして採用（チェスポーン primitive は撤去）。本物の人型 GLB が用意でき次第、`citizen_factory.gd::BODY_NODE` を差し替える。

---

## Sprint 10 適用状況

- [x] 動物: `Animals_FREE.glb` → `deer` を採用（`AnimationPlayer` はパックに含まれないので対象外）
- [x] 資源・木: `simple_nature_pack_glb.glb` → `Plant_01`（berry_bush）、`Oak_Tree_01`（森）
- [x] 資源・水: `Rock_01` を 3 個リング状に配置（Sprint 12.5 で primitive cylinder を撤去）
- [x] 地形: `terrian.glb` を装飾背景として `TerrainBuilder.build_visual_backdrop()` で配置（scale/Y はエディタ実機で調整）
- [x] 住民: `Tree_Stump_01` をプレースホルダとして採用（Sprint 12.5 でチェスポーン primitive を撤去）。本物の人型 GLB 入手まで仮運用。

---

## SFX

### 即使用可（既に組み込み済み）

| パス | 用途 |
|---|---|
| `sfx/Ambience/CampAmbience.mp3` | Sprint 4 BGM（ループ再生中） |

### 採集・インタラクション系

| パス | 用途候補 |
|---|---|
| `sfx/Food & Herbs/Food1–4.mp3` | 採集 SE |
| `sfx/Food & Herbs/Herb1–5.mp3` | 採集 SE（植物） |

### UI・操作系

| パス | 用途候補 |
|---|---|
| `sfx/Misc/Button1–8.mp3` | UI ボタン操作音 |
| `sfx/Coins/Coin1–5.mp3` | 資源入手 SE |
| `sfx/Coins/CoinBag1–7.mp3` | まとめ入手 SE |
| `sfx/Coins/CoinFlip1–10.mp3` | 決定音など |
| `sfx/Coins/CoinPickUp1–2.mp3` | ピックアップ音 |
| `sfx/Coins/CoinPouring1–3.mp3` | 大量獲得 SE |

### アイテム・装備系（将来用）

| パス | 用途候補 |
|---|---|
| `sfx/Bags/BagOpen1–5.mp3` / `BagClose1–4.mp3` / `BagZipper1–5.mp3` | インベントリ操作 |
| `sfx/Books & Scrolls/BookOpen1–3.mp3` / `ScrollOpen1–4.mp3` / `TurnPage1–7.mp3` | 技術解禁 / ログ表示 |
| `sfx/Jewels & Runes/Jewel1–5.mp3` / `Rune1–5.mp3` | 技術解禁 / 特殊イベント |
| `sfx/Clothing/Armor/ArmorEquip1–5.mp3` 他 | 装備関連（Phase 3+） |
| `sfx/Clothing/Clothes/ClothesEquip1–5.mp3` 他 | 衣類関連（Phase 3+） |
| `sfx/Clothing/Leather/LeatherEquip1–5.mp3` 他 | 革製品関連（Phase 3+） |
| `sfx/Weapons/Metal Weapon/MetalWeaponEquip1–5.mp3` 他 | 武器関連（Phase 3+） |
| `sfx/Weapons/Wooden Weapon/WoodenWeaponEquip1–5.mp3` 他 | 木製武器関連（Phase 3+） |

---

## 封印済み（2D 版専用）

キャラクタースプライト・タイルセット一式は 2D 版に戻す場合のみ参照。
3D 移行後は使わない。
