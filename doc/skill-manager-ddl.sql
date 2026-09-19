-- =============================================================
-- Skill Manager — Database DDL (SQLite)
-- 技术选型：Tauri v2 + React (SPA) + SQLite
-- 哈希方案：SHA-256 前 8 字节转 INTEGER（64 位有符号）
--
-- ⚠️ 历史快照（阶段一设计稿）：只含 tools / projects / skills /
--    skill_installations / sync_logs 五张表。**当前 schema 的唯一真源是
--    `src-tauri/src/db.rs` 的建表与迁移逻辑**（另有 markets、remote_skills、
--    skill_conflicts、dismissed_updates、market_templates、projects 扩展列等，
--    并对老库做 pragma_table_info 驱动的 ALTER 迁移）。改表结构请改 db.rs，
--    不要照这份文件推理，也不要把它当作迁移脚本执行。
-- =============================================================

-- 开启外键约束（SQLite 默认关闭）
PRAGMA foreign_keys = ON;

--------------------------------------------------------------
-- 1. tools — 工具注册表
--    id 用工具名称的哈希值，天然去重
--------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tools (
    id              INTEGER PRIMARY KEY,        -- SHA-256(name)[0:8] → INTEGER
    name            TEXT    NOT NULL UNIQUE,     -- 用户输入的工具名，如 "Claude Code"
    global_path     TEXT    NOT NULL,            -- 系统级默认路径，如 ~/.claude/skills/
    project_rel_path TEXT   NOT NULL,            -- 项目相对路径，如 .claude/skills/
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

--------------------------------------------------------------
-- 2. projects — 用户添加的项目
--    id 用项目绝对路径的哈希值
--    全局项目作为特殊内置项目存在（id=0）
--------------------------------------------------------------
CREATE TABLE IF NOT EXISTS projects (
    id              INTEGER PRIMARY KEY,        -- SHA-256(path)[0:8] → INTEGER
    name            TEXT    NOT NULL,            -- 用户起的别名
    path            TEXT    NOT NULL UNIQUE,     -- 项目根目录绝对路径
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_projects_path ON projects(path);

--------------------------------------------------------------
-- 3. skills — Skill 元数据
--    id 用 SSOT 路径的哈希值，天然去重
--    MVP 阶段不分 remote / local，type 字段后置
--------------------------------------------------------------
CREATE TABLE IF NOT EXISTS skills (
    id              INTEGER PRIMARY KEY,        -- SHA-256(source_path)[0:8] → INTEGER
    name            TEXT    NOT NULL,            -- SKILL.md front matter 的 name
    description     TEXT,                        -- SKILL.md front matter 的 description
    source_path     TEXT    NOT NULL UNIQUE,     -- SSOT 目录下的绝对路径
    content_hash    TEXT    NOT NULL,            -- 目录全量文件的 SHA-256
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);

--------------------------------------------------------------
-- 4. skill_installations — Skill 与工具的安装关系
--    全局和项目共用一个张表，通过 project_id 区分
--    project_id = 0 表示全局安装
--------------------------------------------------------------
CREATE TABLE IF NOT EXISTS skill_installations (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    skill_id        INTEGER NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    tool_id         INTEGER NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    project_id      INTEGER NOT NULL DEFAULT 0 REFERENCES projects(id) ON DELETE CASCADE,
    status          TEXT    NOT NULL DEFAULT 'disabled'
                        CHECK (status IN ('active', 'disabled')),
    synced_at       TEXT,                        -- 最后一次同步时间（可为 null）
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),

    -- 同一 Skill 在同一项目的同一工具下只对应一条记录
    UNIQUE (skill_id, tool_id, project_id)
);

CREATE INDEX IF NOT EXISTS idx_inst_skill ON skill_installations(skill_id);
CREATE INDEX IF NOT EXISTS idx_inst_tool ON skill_installations(tool_id);
CREATE INDEX IF NOT EXISTS idx_inst_project ON skill_installations(project_id);
CREATE INDEX IF NOT EXISTS idx_inst_status ON skill_installations(status);

--------------------------------------------------------------
-- 5. sync_logs — 同步操作日志
--    每次实际的同步操作记录一条，用于追溯和排查
--------------------------------------------------------------
CREATE TABLE IF NOT EXISTS sync_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    skill_id        INTEGER NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    tool_id         INTEGER NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
    project_id      INTEGER NOT NULL DEFAULT 0 REFERENCES projects(id) ON DELETE CASCADE,
    direction       TEXT    NOT NULL CHECK (direction IN ('to_ssot', 'from_ssot')),
    status          TEXT    NOT NULL CHECK (status IN ('success', 'failed')),
    error_message   TEXT,                        -- 失败时记录原因
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_log_skill ON sync_logs(skill_id);
CREATE INDEX IF NOT EXISTS idx_log_project ON sync_logs(project_id);
CREATE INDEX IF NOT EXISTS idx_log_direction ON sync_logs(direction);
CREATE INDEX IF NOT EXISTS idx_log_status ON sync_logs(status);
CREATE INDEX IF NOT EXISTS idx_log_created ON sync_logs(created_at);

-- =============================================================
-- Seed Data — 出厂预设数据
-- =============================================================

-- 内置全局项目（id=0 是固定值，不是哈希）
INSERT OR IGNORE INTO projects (id, name, path) VALUES (
    0,
    'Global',
    '~/.agents/skills/'
);

-- 默认工具预设
-- 工具名的哈希值用下面 Python 片段计算后填入
INSERT OR IGNORE INTO tools (id, name, global_path, project_rel_path) VALUES
    (-768412307910267356,  'Claude Code',    '~/.claude/skills/',    '.claude/skills/'),
    (-5387663353590988835, 'Codex CLI',       '~/.codex/skills/',    '.codex/skills/'),
    (8996106060148633658,  'OpenCode',       '~/.opencode/skills/', '.opencode/skills/'),
    (-1843142830140024973, 'Gemini CLI',      '~/.gemini/skills/',   '.gemini/skills/'),
    (6180056807951602058,  'Cline',           '~/.cline/skills/',    '.cline/skills/');
