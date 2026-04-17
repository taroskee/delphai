# アセット在庫

> `game/assets/` に含まれるアセットの一覧。必要になった時に参照する。

---

## 3D モデル（GLB）

| パス | 内容 | 用途候補 |
|---|---|---|
| `animal/Animals_FREE.glb` | 動物パック（フリー） | Phase 2 以降の動物モデル差し替え |
| `animal/Separate_assets_glb.zip` | 個別動物 GLB（展開済み） | 個別アニメーション付き動物 |
| `geography/terrian.glb` | 地形モデル | Phase 2 以降の地形差し替え |
| `nature/simple_nature_pack_glb.glb` | 自然小物パック（木・茂み等） | 資源ノード・装飾 |

> 現在（Phase 1）は Godot primitives（CapsuleMesh / SphereMesh 等）で代替中。  
> Phase 2 でモデル差し替え時に参照する。

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
