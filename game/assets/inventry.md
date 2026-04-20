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
| `Tree_Stump_01` | （Sprint 13.x で住民胴体から外した。チェスポーン primitive に戻したため未使用。） |
| `Tree_Stump_02` | — |

- animations: なし。
- 水源メッシュは含まれていない → `Rock_01` を 3 個リング状に配置して「水辺」を示す（Sprint 12.5 で primitive cylinder を撤去）。

### `geography/terrian.glb` (78 MB) — **Sprint 13.1 で非推奨化**

- Sprint 13.1 以降は使用しない。Terrain3D ProcGen (`TerrainBuilder.build_terrain3d`) が地表生成を担当。
- 物理実体として未使用だった（装飾専用 GLB）ため、削除は次回 MVP クリーンアップで実施。
- 旧仕様参考: Sketchfab 製モノリス (`Object_2..Object_6` / 5 メッシュ / ~57 MB)、`build_visual_backdrop()` がマップ中心に配置していた。

### `animal/Separate_assets_glb.zip`

個別動物の GLB（展開済みの可能性あり）。未調査。Phase 2 以降で個別アニメーションが必要になった時に再評価。

---

## 住民（人型）GLB

**未取得**。所有している人型モデルは Unreal Engine 向けで Godot への変換が未完了。
住民胴体は Godot primitive（CylinderMesh + SphereMesh の 3 パーツ「チェスポーン」）に戻している。
Tree_Stump_01 を一時採用したが（Sprint 12.5）シルエットが scenery に紛れて住民として読めず、Sprint 13.x で primitive に戻した。
本物の人型 GLB が用意でき次第 `citizen_factory.gd::_add_pawn_body` を差し替える。

---

## Sprint 10 適用状況

- [x] 動物: `Animals_FREE.glb` → `deer` を採用（`AnimationPlayer` はパックに含まれないので対象外）
- [x] 資源・木: `simple_nature_pack_glb.glb` → `Plant_01`（berry_bush）、`Oak_Tree_01`（森）
- [x] 資源・水: `Rock_01` を 3 個リング状に配置（Sprint 12.5 で primitive cylinder を撤去）
- [x] 地形: Sprint 13.1 で `terrian.glb` を撤去 → `TerrainBuilder.build_terrain3d()` による FastNoiseLite 高度マップ ProcGen に置換（村 SE は force-flat オーバーレイで平坦化）
- [x] 住民: チェスポーン primitive（cylinder + sphere の 3 パーツ）。Sprint 12.5 で Tree_Stump_01 に置換 → Sprint 13.x で視認性のため primitive に戻した。本物の人型 GLB 入手まで仮運用。

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
