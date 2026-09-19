// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The backend emits these codes from `src-tauri/src/paths.rs`; if a code is
// renamed there, the mapping asserted here has to follow, which is the point of
// pinning the exact strings rather than their shape.

import { describe, it, expect } from "vitest";
import { localizeApiError, makeT } from "./index";

const t = makeT("en");

describe("localizeApiError", () => {
  it("turns each backend path code into reader-facing text", () => {
    expect(localizeApiError(t, "empty-path")).toBe("Path cannot be empty");
    expect(localizeApiError(t, "relative-path:./skills/")).toBe("Please enter an absolute path: ./skills/");
    expect(localizeApiError(t, "env-var-unsupported:%APPDATA%\\skills")).toBe(
      "Environment variables are not supported; enter the full path: %APPDATA%\\skills"
    );
    expect(localizeApiError(t, "path-not-found:C:\\nope")).toBe("Path does not exist: C:\\nope");
  });

  it("leaves anything it does not recognise untouched", () => {
    expect(localizeApiError(t, "Failed to add project: database is locked")).toContain("database is locked");
  });

  it("follows the active language", () => {
    expect(localizeApiError(makeT("zh"), "empty-path")).toBe("路径不能为空");
  });
});
