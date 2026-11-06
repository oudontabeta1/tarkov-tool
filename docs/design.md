# Escape from Tarkov サポートアプリ 設計書

最終更新: 2026-11-01
状態: 調査完了・実装前(データ構造の確認と試作パーサの検証まで完了)

---

## 1. 目的と前提

Escape from Tarkov のプレイを補助する**独立したデスクトップアプリ**。ゲーム画面へのオーバーレイは行わず、別ウィンドウ(サブモニター等)で使う。

| 項目 | 決定内容 |
|---|---|
| 形態 | デスクトップアプリ(オーバーレイなし) |
| 技術 | **Tauri**(Rust + Web UI)。対象OSは Windows / macOS |
| データ元 | **escapefromtarkov.fandom.com**(MediaWiki API 経由) |
| 運用コスト | できるだけ 0 円(サーバー不要、すべてローカル完結) |
| 負荷方針 | Wiki全体は取得しない。必要なページだけ、差分更新で取得する |

### 要件

1. **タスク管理**
   - 全タスクを一覧し、マップ・タスクポイント・必要アイテム・報酬を分かりやすく表示
   - 名前検索、タグ付け、「今やる予定」、「そのレイドで終わった」の識別
2. **アイテム仕様**
   - 弾薬の性能と対応武器、食料(・医薬品)の性能をすぐ確認できる
3. **データ取得**
   - ほぼ全てWiki経由。スクレイピングで最新かつ正確な情報を使う

---

## 2. 全体設計

### 2.1 設計の柱

1. **必要なページだけ取得する**: Wikiは全体で31,709ページ(記事は6,209)。全部は取らず、索引ページ経由で対象だけを取る
2. **差分更新**: 前回同期以降に変更されたページだけ再取得する
3. **UIは常にローカルSQLiteだけを読む**: 通信中でも画面は固まらず、オフラインでも使える

### 2.2 構成

```
┌─────────── Tauri アプリ ───────────┐
│ Web UI (React + TypeScript + Vite)  │
│  仮想スクロール一覧 / 検索 / フィルタ │
│        ↕ invoke / event             │
│ Rust バックエンド                    │
│  ├ wiki クライアント(API取得)        │
│  ├ parser(wikitext → 構造体)         │
│  ├ sync(差分判定・進捗通知)          │
│  └ db(SQLite)                       │
└──────┬───────────────┬──────────────┘
       │               │
  wiki.db(再生成可)   user.db(消さない)
       │
  Fandom API: https://escapefromtarkov.fandom.com/api.php
```

| 層 | 採用 | 備考 |
|---|---|---|
| UI | React + TypeScript + Vite | 一覧は `@tanstack/react-virtual` で仮想化 |
| 通信 | Rust `reqwest` + `tokio` | 同期は別スレッド。進捗は `emit` でUIへ |
| DB | `rusqlite`(bundled) | 検索は FTS5(全文検索) |
| パーサ | **Rust で実装**(既定) | 同期中のUI固まりを避けるため。TypeScript案も可 |

### 2.3 DBを2ファイルに分ける

- `wiki.db`: Wiki由来のキャッシュ。壊れても作り直せる
- `user.db`: 進捗・タグ・設定。再同期やスキーマ変更の影響を受けない

ユーザーデータの `task_id` は**Wikiのページタイトル由来の安定ID**(スラッグ化)にする。Wiki側の再生成で行が作り直されても、紐付けが切れないため。

---

## 3. Wikiの構造(調査結果)

### 3.1 サイト規模

| 項目 | 値 |
|---|---|
| 全ページ | 31,709 |
| 記事 | 6,209 |
| 編集数 | 360,127 |
| Module名前空間 | 14件(汎用部品のみ。**データを持つLuaモジュールは無い**) |

### 3.2 テンプレートの種類

| 種類 | 例 | 役割 |
|---|---|---|
| Infobox 系 | `Infobox quest` / `ammo` / `weapon` / `item` / `key` / `magazine` / `gear` / `throwable` / `location` / `character` / `skill` | ページ冒頭の情報欄。`Infobox` は汎用レイアウト部品で、データは各個別テンプレートの引数 |
| Navbox 系 | `Navbox quests` / `rounds` / `provisions` / `medical` / `weapons` / `magazines` / `keys` ほか | **カテゴリ別の索引**。全体像を数ページで取れる |
| 口径テンプレート | `Template:5.45x39mm` など(Category: Ammo templates, 34件) | `*{{弾薬ID}}` の箇条書き |
| ID テンプレート | `Template:56dfef82d2720bbd668b4567` など | **リンク1本だけ**(例: `[[Weapon]]`)。BSGアイテムIDとページ名の対応表。Category: Link templates |

> 注意: 弾薬の取得では、口径ページと `Navbox rounds` がページ名を直接持つため、IDテンプレートの解決は不要。IDは将来 tarkov.dev 等と突き合わせる際の鍵になりうる(未検証)。

### 3.3 カテゴリ件数

| カテゴリ | 件数 | 備考 |
|---|---|---|
| Quests | 937 | **Navbox quests は536件**。差の約400件はイベント・アリーナ・廃止等と見られる(未精査) |
| Ammo packs | 221 | |
| Ammo templates | 34 | 口径テンプレート |
| Weapons | 229 | |
| Food / Drinks / Provisions | 26 / 20 / 3 | |
| Medical supplies / treatment | 19 / 5 | |
| Traders | 12 | |
| Maps | 6 | ナビ欄の紹介テンプレートでは13マップ |

### 3.4 マップ一覧(ナビ欄より、13件)

Customs / Factory / Ground Zero / Icebreaker / Interchange / Lighthouse / Reserve / Shoreline / Streets of Tarkov / Terminal / The Lab / The Labyrinth / Woods

---

## 4. データ種別ごとの取得方法とパース規則

### 4.1 タスク

**索引**: `Template:Navbox quests`(トレーダー別に全タスクが `[[Title]]` で並ぶ。536件)

- トレーダー: Prapor / Therapist / Fence / Skier / Peacekeeper / Mechanic / Ragman / Jaeger / Ref / Lightkeeper / BTR Driver
- 表記ゆれ: `[[Immunity (quest)|Immunity]]`、曲がったアポストロフィ(`’`)、末尾の空白、`Quests#Operational_Tasks`(タスクではないので除外)

**各ページの構造**

```
{{Infobox quest
|given by        = [[Prapor]]
|location        = [[Customs]], [[Ground Zero]], ...
|LL requirement  = 1          ← トレーダー忠誠度(プレイヤーレベルではない)
|LL quests       = ...
|previous        = [[A]]<br/>or<br/>[[B]]     ← 「どれか」表記あり
|leads to        = ...
|related / related2 / reqkappa  ← テンプレート未定義の引数もある
}}
==Dialogue==   ==Requirements==(任意)  ==Objectives==  ==Rewards==  ==Guide==
```

| 抽出項目 | 取得元 | 方法 |
|---|---|---|
| 依頼人・マップ・忠誠度・前提・後続 | Infobox 引数 | 引数パース + リンク抽出 |
| Kappa要否 | `reqkappa`(ある場合) | `Yes` / `No` |
| プレイヤーレベル要件 | `==Requirements==` の文章 | 正規表現 `level (\d+)`(無いページあり) |
| 前提タスク(詳細) | `==Requirements==` の入れ子箇条書き(`**`) | `previous` が `See requirements` の場合はこちら |
| 目標 | `==Objectives==` の箇条書き | **元の文を必ず保持**。リンクからマップ・アイテム、文中の数字から数量、`in raid` からFIRを補助抽出 |
| 報酬 | `==Rewards==` の最上位箇条書き | 下表 |
| 必要アイテム表 | `==Guide==` 内の「Related Quest Items」表(あるページのみ) | アイテム名・数量・要件・FIR |

**報酬の書式と抽出**

| 書式 | 例 | 抽出 |
|---|---|---|
| EXP | `+3,000 [[EXP]]` | 数値 |
| 評価 | `[[Prapor]] Rep +0.1` | 文字列 |
| ルーブル | `80,000 Roubles`(入れ子の `**` 行はIntelligence Center加算なので別扱い) | 文字列 |
| アイテム | `1× [[IOTV Gen4 ...]]` | 数量 + 名前 |
| その他 | `Unlocks achievement ...` | 文字列で保持 |

**目標の例**

- `Eliminate 7 [[Scavs]] in one raid on [[Ground Zero]]`
- `Find 3 [[Salewa first aid kit]] [[Found in raid|<font color="red">in raid</font>]]`
- `Hand over the found <font color="red">in raid</font> item: [[Antique axe]]`

### 4.2 弾薬

| 取得先 | 内容 |
|---|---|
| **口径ページ**(例: `5.45x39mm`) | `==Types==` の表に全弾薬の数値(DMG、貫通、アーマーダメージ%、精度、反動、出血率、耐久消費、発熱、弾速、入手元)。**33口径×1ページ** |
| `Ammunition` ページ | 口径 → 対応武器(種別つき: Assault rifles / SMGs / Pistols 等)。1ページで全口径 |
| 個別弾薬ページ(`Infobox ammo`) | 口径ページに無い項目の補完(発射数 `projectiles`、`effects`、ricochet など)。必要時のみ |
| `Navbox rounds` | 口径 → 弾薬ページ名の索引(`[[5.45x39mm BS gs|BS]]` 形式) |

注意点:

- 数値セルは `<font color="green">+50</font>` のように色付きの場合がある(色タグを除去して数値化)
- `5.8x42mm` と `5.8×42mm` のように、表記違いの重複テンプレートがある
- 弾薬名の `<sup>T</sup>`(曳光弾)・`<sup>S</sup>`(亜音速弾)は属性として保持する
- 散弾は `projectiles`(発射数)が重要

### 4.3 食料

- `Provisions` ページの表: Energy、Hydration、Buffs、Debuffs、Use Time、物々交換可否、クラフト可否、Notes
- Notes に「タスクで必要な数・FIR」(例: `1 needs to be found in raid for the quest Collector`)があり、**アイテム ↔ タスクの紐付け**に使える
- 試作での検証: 45品を抽出

### 4.4 医薬品

- `Medical` ページ: 「Medical items」表(9列: 名前・種類・バフ・デバフ・使用時間・使用回数・HPプール・1回の最大回復)と「Injectors and stimulants」表(6列)
- バフ・デバフは文章+アイコン(`[[File:Pain.png|46px|Pain]]`)。**数値化せず、テキストとタグで保持**(アイコンの代替名を残す)
- 試作での検証: 43品(医療品21 + 注射・刺激剤22)

### 4.5 武器(後続フェーズ)

- `Infobox weapon` に `caliber`、`ammo`、`def ammo`、`def mag` などがある
- ページタイトルが長い(例: `Kalashnikov AK-74N 5.45x39 assault rifle`)。索引上の短い名前はリダイレクトなので、取得時は `redirects=1` を付ける
- 口径 → 武器の対応は `Ammunition` ページで足りるため、初期版では武器ページを取らない

---

## 5. 取得(同期)戦略

### 5.1 取得量の見積もり

| 対象 | 方法 | リクエスト数(目安) |
|---|---|---|
| 索引・一覧(`Navbox quests`、`Ammunition`、`Provisions`、`Medical` ほか) | 直接取得 | 1〜2 |
| タスク約536件 | 50件/リクエスト | 約11 |
| 弾薬 | 口径ページ33枚 | 1(50件まで一括) |
| 個別アイテム・武器 | 詳細を開いたとき | オンデマンド |

初回同期は合計約15リクエスト。1秒間隔でも30秒前後の見込み。

### 5.2 更新

- 起動時または手動の「更新」ボタンで、`list=recentchanges` を前回同期時刻以降で取得し、キャッシュ済みページだけ再取得する
- 週1回程度、`revid` による全体の整合確認を行う(任意)

### 5.3 通信ルール

- 直列で、1秒程度の間隔を空ける
- `maxlag` パラメータを付け、サーバーが混んでいるときは待つ
- User-Agent にアプリ名と連絡先を入れる
- `redirects=1` を付ける
- 画像は初期版では取得しない(必要になったら表示時に遅延ロード)

### 5.4 失敗時の扱い

- `wiki_pages` テーブルに `title / revid / fetched_at / parse_status / error_msg` を保存
- パース失敗は握りつぶさず `parse_status = error` で記録し、設定画面で一覧表示
- 解釈に失敗した目標は元の文をそのまま表示(画面が空にならない)

---

## 6. データモデル

### 6.1 wiki.db(Wiki由来・再生成可)

```
wiki_pages    (title PK, revid, fetched_at, parse_status, error_msg)

maps          (id, name)
traders       (id, name)

tasks         (id, name, trader_id, wiki_title, loyalty_level, player_level,
               kappa, requirements_text, previous_mode)   -- previous_mode: all / any
task_prereqs  (task_id, prereq_task_id, group_no)         -- group_no: 「or」の束ね
task_maps     (task_id, map_id)
objectives    (id, task_id, order_no, kind, text, map_id, qty, fir)
objective_items (objective_id, item_name, qty, fir)
rewards       (id, task_id, kind, item_name, qty, text)   -- exp/rep/roubles/item/other

items         (id, name, category, wiki_title)
ammo          (item_id, caliber, dmg, pen, armor_dmg, accuracy, recoil,
               light_bleed, heavy_bleed, dur_burn, heat, speed,
               projectiles, tracer, subsonic, source, effects)
weapons       (item_id, name, class)
ammo_weapon   (caliber, weapon_name, weapon_class)
provisions    (item_id, kind, energy, hydration, buffs, debuffs, use_time,
               barter, craft)
medical       (item_id, group, type, buffs, debuffs, use_time, uses,
               hp_pool, max_heal)
item_quest_use (item_name, task_id, qty, fir)             -- Notes/Guide表から

items_fts / tasks_fts   (FTS5)
```

### 6.2 user.db(消さない)

```
task_state  (task_id PK, status, pinned_at, updated_at)
            -- status: todo / planned(今やる) / done
tags        (id, name, color)
task_tags   (task_id, tag_id)
raid_log    (id, map_id, started_at, ended_at)
raid_done   (raid_id, task_id)     -- そのレイドで完了したタスク
settings    (key, value)
```

---

## 7. 機能・画面

### 7.1 タスク

- 一覧(カード / 表の切替)
- 左にフィルタ(マップ、トレーダー、状態、タグ)、上に名前検索
- 詳細(右ペイン): 目標、必要アイテム(納品 / FIR)、報酬、前提・後続タスク
- 状態: 未着手 / **今やる(ピン留め)** / 完了
- 自由タグ(例: 「キー必要」「協力」「後回し」)
- 前提タスク未完了のものは「未解放」と判別できる表示

### 7.2 レイドモード

- マップを選ぶと、そのマップの「今やる」タスクだけを大きく表示
- レイド終了時に、**このレイドで終わったタスク**をまとめてチェックし、完了にする

### 7.3 アイテム

- 弾薬: 口径別の比較表(DMG/貫通でソート)、対応武器の表示
- 武器 → 使える弾薬、弾薬 → 使える武器の相互参照
- 食料・医薬品の効果一覧(Energy / Hydration / バフ / 使用時間など)
- アイテムページから、必要とするタスクへ移動(`item_quest_use`)

### 7.4 検索

- タスク・アイテム・弾薬を横断する1つの検索ボックス(FTS5。英語名中心)

### 7.5 設定

- 同期ボタン、最終同期時刻、パース失敗一覧
- ライセンス表記(Wikiのテキスト利用に伴う CC BY-SA の表記)

---

## 8. ノーコスト運用

- サーバー不要。取得・保存・表示がすべてローカル
- Fandom API、Tauri、SQLite はいずれも無料
- 配布する場合は GitHub Releases + GitHub Actions(公開リポジトリは無料)。コード署名は有料のため、署名なしだと Windows SmartScreen と macOS の未署名警告が出る(個人利用なら問題なし)
- 発展案: 他の人にも配る場合、GitHub Actions で定期的にWikiを取得して整形済みJSONを作り、Releases に置く。各アプリはそのJSONだけダウンロードし、Wikiへの負荷をほぼゼロにする(個人用なら不要)

---

## 9. リスクと未確認事項

| 項目 | 内容 | 対処 |
|---|---|---|
| 目標の自然文 | 数量・マップ・アイテムの抽出が失敗しうる | 元の文を保持。抽出は補助 |
| プレイヤーレベル要件 | infoboxに欄が無く、書いていないページもある | Requirements節の文章から抽出。無ければ空 |
| 前提タスク表記 | `A or B or C`、`See requirements` など | `previous_mode` と `group_no` で表現 |
| 索引の差 | Category(937)とNavbox(536)の差約400件が未精査 | 既定はNavbox。残りは「その他のタスク」として後から追加 |
| Wikiの更新 | テンプレートや書式が変わるとパーサが壊れる | 失敗一覧で検知。サンプルをテストデータとして保持 |
| 武器ページ | 未取得のため構造は `Infobox weapon` の定義のみ確認 | 後続フェーズで検証 |
| 数値セルの装飾 | 色タグ付きなど | 色タグ除去後に数値化(試作で検証済み) |
| API制限値 | Fandomの公式レート制限は未確認 | 控えめな間隔(1秒)+`maxlag` |
| ID ↔ 外部API | BSGのIDとtarkov.devのIDが一致する可能性は未検証 | 必要になった時点で検証 |
| 他のページ | Hideout、Traders、Keys、Weapon mods などは未調査 | 後続フェーズ |

---

## 10. 実装フェーズ

1. **雛形**: Tauri + React + TypeScript、DB2ファイル、マイグレーション
2. **同期処理**: クライアント(間隔・`maxlag`)、索引取得、差分更新、進捗表示
3. **パーサ**: タスク → 弾薬 → 食料 → 医薬品(調査で集めたサンプルをテストデータにする)
4. **タスクUI**: 一覧・検索・フィルタ・状態・タグ
5. **レイドモード**
6. **アイテムUI**: 弾薬比較表、対応武器、食料・医薬品
7. **仕上げ**: 横断検索、エラー一覧、ライセンス表記、差分更新の運用

### 決める必要があること

- パーサの実装言語(**既定は Rust**。TypeScript にすると書きやすいが、同期中のUI負荷が増える)

---

## 付録A: 調査用スクリプトと試作パーサ

- `probe.py`(調査用): MediaWiki API で `siteinfo`、カテゴリ、テンプレート、ページ本文を取得して保存する(`discover` / `pages` / `cat` / `templates` / `categories` / `follow` / `bundle`)
- `wiki_parsers_prototype.py`(試作パーサ): Python 製。以下を検証済み
  - タスク: Debut / Shortage / Collector / Gunsmith - AKM / Stabilize Business
  - 弾薬: `5.45x39mm` の口径表、`5.45x39mm BS gs`、`12/70 flechette`
  - `Ammunition`(33口径・対応武器)、`Provisions`(45品)、`Medical`(43品)
- 本実装では、このパーサをRustに移植し、同じサンプルをテストデータにする

## 付録B: 主なAPI呼び出し

| 目的 | パラメータ |
|---|---|
| 本文の一括取得(50件まで) | `action=query&prop=revisions&rvprop=ids\|timestamp\|content&rvslots=main&redirects=1&titles=A\|B\|...` |
| 変更の確認 | `action=query&list=recentchanges&rcend=<前回同期時刻>&rcnamespace=0\|10&rclimit=500` |
| カテゴリの全件 | `action=query&list=categorymembers&cmtitle=Category:Quests&cmlimit=500&cmtype=page`(`continue` でページ送り) |
| 共通 | `format=json&formatversion=2&maxlag=5` と、アプリ名・連絡先入りの User-Agent |
