# Tarkov Helper

Escape from Tarkov のサポート用デスクトップアプリ(Tauri 2 + React + Rust)。ゲーム画面へのオーバーレイは行わず、別ウィンドウで使います。
データは [Escape from Tarkov Wiki](https://escapefromtarkov.fandom.com)(Fandom)から取得し、ローカルのSQLiteに保存します。サーバーは不要で、運用コストはかかりません。

## できること

| 画面 | 内容 |
|---|---|
| タスク | 全タスク(約536件)の一覧。名前・トレーダー(アイコン)・マップ・状態・タグで絞り込み。詳細に目標、必要アイテム(FIR表示)、報酬、前提/後続タスク、**参照Wikiのリンク**、**マップを開くボタン**、報酬のアイコン表示(EXP・トレーダー評価・金額・スキル・アイテム・解放)。状態は「未着手 / 今やる / 完了」、前提が未完了のタスクは「未解放」。一覧の各行の **「+」** で「今やる」に追加/解除 |
| レイド | マップを選ぶと、そのマップの「今やる」タスクを大きく表示。終わったものにチェックして「レイド終了」で一括完了 |
| マップ | タスクの目標の位置(到達・マーク・設置・アイテム入手など)を、マップ上にまとめて表示。「今やる」「未完了すべて」「すべて」「選んだ1件」で表示を切り替え、タスクごとに色分け。地点をクリックすると、そのタスクの詳細へ移動できます。タスク詳細の「📍 地図で見る」、レイド画面の「📍 …の地点を地図で見る」からも開けます |
| アイテム | アイコン付き。弾薬(口径別、列ごとにソート、使える武器、武器名から使える弾薬を逆引き)、食料・飲料、医薬品・注射 |
| 設定 | Wikiとの同期、連絡先(User-Agent用)、取得に問題があったページ、ライセンス表記 |

## フォント(Bender)

UIのフォントは [Bender](https://www.cufonfonts.com/font/bender) を指定しています(ライセンスの都合で同梱していません)。
ダウンロードしたファイルを `public/fonts/` に置いてください(ファイル名などは `public/fonts/README.txt` を参照)。
PCにインストール済みならそのまま使われ、Benderに無い文字(日本語など)はシステムのフォントで表示されます。

## 操作

| 操作 | 内容 |
|---|---|
| マウスの戻る/進むボタン、Alt+←/→、画面上部の ◀ ▶ | ブラウザのように、直前に見ていたタスクやタブへ戻る/進む |
| Ctrl+K(または `/`) | タスク・弾薬・食料・医薬品を横断する検索 |
| 検索のあいまい一致 | `bgc` → Background Check(文字の飛ばし一致)、`back check`(単語の先頭一致)、`bc`(頭文字)、`prapor`(トレーダー名)、`salewa`(目標・必要アイテム・報酬の中の名前)も検索対象 |
| タスク詳細の「A−」「A+」 | 詳細の文字サイズを変更(保存されます) |
| 「🗺 マップ名」ボタン | Wikiのインタラクティブマップ(`Map:マップ名`)を別ウィンドウで開く |

## セットアップ

必要なもの: [Node.js](https://nodejs.org/) 20+、[Rust](https://rustup.rs/)(stable)、Tauri の前提環境
(Windows: Microsoft C++ Build Tools と WebView2 / macOS: `xcode-select --install`。詳細は <https://tauri.app/start/prerequisites/>)

```bash
npm install
npm run tauri dev      # 開発起動
npm run tauri build    # インストーラ作成(src-tauri/target 以下、または target/release/bundle)
```

初回起動はデータが空なので、右上の **「Wikiから同期」** を押してください(約1分。以降は更新されたページだけ取得します)。
設定画面でメールアドレスやGitHubのURLを連絡先として登録しておくと、Wikiへのアクセス時のUser-Agentに含まれます(任意)。

データは OS のアプリデータフォルダに保存されます。

- `wiki.db`: Wiki由来のキャッシュ。消しても再同期で復元できます
- `user.db`: タスクの状態・タグ・レイド履歴・設定。**消さないでください**

## 構成

```
core/         Rust ライブラリ(UI非依存): wikitext パーサ・SQLite・Fandom同期
  src/wikitext.rs         wikitext の読み取り補助(表・infobox・節・リンク)
  src/parsers/quest.rs    タスク(Infobox quest / Objectives / Rewards / Requirements / Guide)
  src/parsers/items.rs    弾薬(口径ページ・Ammunition)/ 食料 / 医薬品
  src/sync.rs             MediaWiki API クライアントと差分同期
  src/db.rs               wiki.db / user.db
  tests/                  実ページのサンプル(tests/fixtures)を使ったテスト
src-tauri/    Tauri アプリ(コマンド定義、同期の起動と進捗イベント)
src/          React UI
docs/         設計書とPythonの試作パーサ
```

### Windows でリンカのエラーが出るとき

`link: extra operand ...` というエラーは、Visual Studio の C++ ビルドツールが無く、Git/BusyBox/msys2 の別の `link.exe` が使われているときに出ます。
「Build Tools for Visual Studio」のワークロード「C++ によるデスクトップ開発」を入れ、ターミナルを開き直して `cargo clean` してからやり直してください。
(それでも直らなければ「x64 Native Tools Command Prompt for VS」から実行します。)

## テスト

```bash
cargo test -p tarkov-core --lib --tests
```

`core/tests/fixtures/` は、実際のWikiから取得したページのwikitextです(Debut / Shortage / Collector / Gunsmith - AKM / Stabilize Business / 5.45x39mm / Ammunition / Provisions / Medical など)。
Wikiの書式が変わってパースが壊れた場合は、該当ページを同じ形式で差し替えて、テストを直してから、パーサを直してください。

## マップ表示のデータ

Wikiのインタラクティブマップには、タスク単位の座標が入っていません(出口・スポーン・鍵・コンテナなどの地点のみ)。そのため、**目標の位置だけ**は [tarkov.dev](https://tarkov.dev) の公開データ(`json.tarkov.dev` の `tasks` / `tasks_en`)から取得し、タスクの `wikiLink` でWikiのタスクと紐付けます。
地図の画像と座標変換の値は、tarkov-dev リポジトリの `maps.json` を使います(座標は `[z, x]` の順で、マップごとの回転と `transform` を適用)。

- 座標を持つのは、APIのタスク約500件のうち約235件です。「Scavを7体倒す」のような目標には、地点がありません
- 取得に失敗しても、Wikiからの同期は止まりません。状態は設定画面で確認できます(座標を持つタスク数、Wikiと一致しなかった例、前回のエラー)
- 地図画像(SVG)はtarkov.devのサーバーから表示します。オフラインでは地図の背景が出ません(地点の位置は出ます)
- tarkov.dev のGraphQL API(`api.tarkov.dev/graphql`)は停止しているため、使っていません

## 同期の仕組み

1. 索引ページ(`Template:Navbox quests`、`Ammunition`、`Provisions`、`Medical`)を取得
2. タスク約536件と弾薬の口径ページ約33件について、`lastrevid`(最新リビジョン)だけを50件ずつ確認し、**変更があったページだけ**本文を取得
3. パースして `wiki.db` に保存。パースに失敗した・想定外のページは設定画面の一覧に出ます

リクエストは1秒間隔、`maxlag` 付き、混雑・429・5xx のときは待って再試行します。

## 設計書からの変更点(この版の範囲)

- DBは、パース結果をJSONで1行ずつ保存する簡易構成です(設計書の正規化テーブルではありません)。検索・絞り込みは件数が小さいためUI側で行い、FTS5は使っていません
- 変更検出は `recentchanges` ではなく、ページごとの `lastrevid` の比較です(取りこぼしが起きにくいため)
- アイコンは、同期のときにWikiから画像をダウンロードして `wiki.db` に保存し、アプリ内の `icon://` から表示します(オフラインでも表示できます)。タスクの報酬・必要アイテム(武器・防具・鍵など)のアイコンは、各アイテムのWikiページの infobox から取得します。保存できなかった画像は、Wikiの画像URLの直接表示を試し、それも失敗すると空欄になります。状態は設定画面の「アイコンの状態」で確認できます。トレーダーのアイコンは各トレーダーのページの infobox(`icon`、無ければ `image`)から取得し、見つからない場合は頭文字で表示します
- 未実装: 武器・個別アイテムページ、tarkov.dev による補完、`Category:Quests` にのみ存在するイベント/アリーナ等のタスク(約400件)、目標ごとの進捗チェック

## 注意

- 非公式のファンツールで、Battlestate Games とは関係ありません
- Wikiの内容は CC BY-SA のもとで提供されています。再配布する場合は表記を残してください
