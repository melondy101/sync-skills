<p align="center">
  <a href="README.md">中文</a> | <a href="README.en.md">English</a> | <strong>日本語</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.2.1-blue?style=flat-square" alt="version">
  <img src="https://img.shields.io/badge/Tauri-v2-orange?style=flat-square&logo=tauri" alt="tauri">
  <img src="https://img.shields.io/badge/Rust-2021-brown?style=flat-square&logo=rust" alt="rust">
  <img src="https://img.shields.io/badge/React-19-blue?style=flat-square&logo=react" alt="react">
  <img src="https://img.shields.io/badge/license-AGPL--3.0-green?style=flat-square" alt="license">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey?style=flat-square" alt="platform">
</p>

<h1 align="center">⬡ Skill Manager</h1>

<p align="center">
  <strong>AI コーディングツール間での Skill 同期マネージャー</strong><br>
  一度編集すれば、すべてのツールに反映。
</p>

<p align="center">
  <a href="#機能">機能</a> •
  <a href="#アーキテクチャ">アーキテクチャ</a> •
  <a href="#インストール">インストール</a> •
  <a href="#開発">開発</a> •
  <a href="#ロードマップ">ロードマップ</a>
</p>

---

## なぜ Skill Manager が必要か

AI コーディングアシスタント（Claude Code、Cursor、Windsurf、Cline など）は、すべて `SKILL.md` ファイルを使って再利用可能な知識やワークフローを定義します。複数のツールを併用している場合、Skill ファイルはあちこちのディレクトリに散在し、手動での同期は手間がかかり、ミスも起こりがちです。

Skill Manager はデスクトップ GUI を提供し、すべての Skill を一箇所で管理し、設定されたすべての AI ツールに自動同期します。

## 機能

- **ツール管理** — AI コーディングツールのパスを登録。13 以上の既知ツールを自動検出
- **Skill スキャン** — ディレクトリを再帰的にスキャンし、`SKILL.md` を含むスキルディレクトリをすべて特定
- **SSOT 同期** — `~/.skill-manager/ssot/` を中心としたハブ＆スポークモデルで各ツールに配布
- **リバース同期** — SSOT から指定ツールディレクトリへプッシュし、ローカル変更を上書き
- **コンフリクト管理** — ツール間のバージョンコンフリクトを検出し、diff プレビューと解決を提供
- **変更の却下** — 特定のツールの変更を永続的に無視。内容が再度変更されるまで再通知しない
- **プロジェクト別管理** — プロジェクトごとに独立した Skill セットを構成、編集対応
- **差分検出** — LCS diff ビュー（並べて表示 / 統合の 2 モード）を内蔵し、ファイル単位の変更を正確に表示
- **Skill マーケット** — GitHub / GitLab リポジトリからスキルを閲覧・検索・ワンクリックインストール。マーケットソース管理と一括操作に対応
- **ファイル監視** — SSOT ツリーを監視し、ディスク変更時に自動同期（full-auto）または通知（semi-auto）
- **アプリ内アップデート** — 新バージョンの検知・ダウンロード・インストールをワンクリックで実行
- **オンボーディングとヘルスチェック** — 初回起動ウィザード、Lint による自動修復付きヘルスチェック、SKILL.md 内蔵エディタ
- **キーボードショートカット** — 検索 / 移動 / 同期 / ソース管理をグローバルに、`?` で早見表を表示
- **テーマ切替と多言語** — ライト / ダーク / システム追従、中文 / English / 日本語
- **アクティビティログ** — すべての操作の完全な監査記録

## アーキテクチャ

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Claude Code │     │   Cursor    │     │  Windsurf   │
│  .claude/    │     │  .cursor/   │     │  .windsurf/ │
└──────┬───────┘     └──────┬──────┘     └──────┬──────┘
       │                    │                    │
       │     Skill Manager (Tauri Desktop)      │
       │         ┌──────────────────┐           │
       └────────►│ ~/.skill-manager/│◄──────────┘
                 │ ssot/ (SSOT Hub) │
                 └────────┬─────────┘
                          │
                 ┌────────┴─────────┐
                 │  skill-manager.db │
                 │  (SQLite)        │
                 └──────────────────┘
```

**技術スタック：** Tauri v2 (Rust + React 19 + TypeScript + SQLite)

メイン UI は 3 つのタブで構成：**Global**（グローバル Skill 管理）、**Projects**（プロジェクト別 Skill 管理）、**Market**（リモート Skill マーケット）。
右上から **Settings** と **Logs** パネルにアクセス可能。アプリはトレイに最小化対応。

## インストール

### ソースからのビルド

**前提条件：**

- [Rust](https://www.rust-lang.org/tools/install)（2021 edition）
- [Node.js](https://nodejs.org/) >= 22 と [pnpm](https://pnpm.io/) >= 11（**pnpm を使用してください。npm / yarn は使わないでください**）
- [Tauri の前提条件](https://v2.tauri.app/start/prerequisites/)

**ビルド手順：**

```bash
git clone https://github.com/huang-yi-dae/sync-skills.git
cd sync-skills
pnpm install
pnpm tauri build
```

ビルド成果物（インストーラー）は `src-tauri/target/release/bundle/` に出力されます。

### 開発モード

```bash
pnpm install
pnpm tauri dev
```

### リリース

各プラットフォームのインストーラーは GitHub Actions で自動ビルドされ、**ローカルでのパッケージングは不要**です：

1. `main` が最新であることを確認；
2. バージョンタグを push：

   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

3. CI（`.github/workflows/release.yml`）が Windows / macOS / Linux の 3 ランナーで
   `.msi` / `.nsis.exe`、`.dmg` / `.app`（universal）、`.deb` / `.AppImage` をビルドし、
   **GitHub Release のドラフト**にまとめます；
4. Releases ページでドラフトを公開します。

> 現在インストーラーにはコード署名されていないため、Windows / macOS で SmartScreen / Gatekeeper の警告が出ます。

**注意**：README のバージョンバッジは最新の公開タグを示します。`main` 上の未公開コミットは [docs/CHANGELOG.md](docs/CHANGELOG.md) の Unreleased 節を参照してください。

## 開発

```
sync-skills/
├── src/                    # フロントエンド (React + TypeScript)
│   ├── App.tsx             # メインコンポーネント & ルーティング
│   ├── App.css             # スタイル（CSS 変数テーマシステム）
│   ├── api.ts              # Tauri IPC ラッパー
│   ├── i18n/               # 多言語テキスト
│   ├── types.ts            # TypeScript 型定義
│   ├── main.tsx            # エントリーポイント
│   ├── components/         # UI コンポーネント
│   └── hooks/              # カスタムフック
├── src-tauri/src/          # Rust バックエンド
│   ├── lib.rs              # エントリ & コマンド登録
│   ├── commands/           # Tauri コマンド層
│   ├── ops/                # ドメインロジック
│   ├── sync.rs             # ファイル同期
│   ├── scanner.rs          # ディレクトリスキャン
│   ├── diff.rs             # LCS 差分アルゴリズム
│   ├── db.rs               # SQLite データアクセス
│   ├── lock.rs             # LockManager
│   ├── lint.rs             # SKILL.md ヘルスチェック
│   ├── market.rs           # リモートマーケット
│   ├── mcp.rs              # 外部向け MCP stdio サーバー（読み取り専用）
│   ├── watcher.rs          # スキルディレクトリのファイル監視
│   └── ...
├── doc/                    # 計画ドキュメント（PRD、設計、フェーズ計画）
├── docs/                   # 問題トリアージ、UI レビュー、マーケット改造ドキュメント
└── .github/workflows/      # CI: verify.yml + release.yml
```

### 検証

コミット前の完全な検証チェックリスト（型チェック / lint / ユニットテスト、フロントエンド + Rust で全 6 コマンド）は [CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。CI（`.github/workflows/verify.yml`）が push / PR ごとに同じチェックを強制実行します。

## ロードマップ

| バージョン | 状態 | 内容 |
|-----------|------|------|
| v0.1.0 | ✅ 完了 | コア機能：ツール管理、Skill スキャン、SSOT 同期、プロジェクト管理 |
| v0.2.0 | ✅ 完了 | 自動検出、ソート/フィルター、差分検出、diff ビュー |
| v0.3.0 | ✅ 完了 | テーマ切替、多言語対応、ハッシュ安定性修正 |
| v0.4.0 | ✅ 完了 | 名前をアイデンティティとして、コンフリクト検出/解決、タイムスタンプ、プロジェクト編集、リバース同期、変更却下 |
| v0.5.0 | ✅ 完了 | LockManager 統合、core_hash 変更検出 |
| v0.6.0 | ✅ 完了 | アプリ内アップデート、オンボーディングウィザード、ヘルスチェック/Lint、内蔵エディタ、Market の一括操作、ファイルウォッチャーによる自動同期、GitLab マーケットソース、全ダイアログのフォーカストラップ/アクセシビリティ統一、グローバルキーボードショートカットと `?` 早見表、Market リストの描画性能 |
| v0.7.0 | 🟡 進行中 | MCP 統合：最初のスライス実装済み —— `skill-manager-mcp` 読み取り専用 stdio サーバー（GUI と同じ SQLite インデックス上の 9 ツール）；MCP サーバー設定の管理とツール間同期はこれから |

> 上のバージョン番号はロードマップ上のステージ名で、公開されているパッケージバージョン（最新タグは `v0.2.1`）とは独立しています。

## ライセンス

[AGPL-3.0](LICENSE)
