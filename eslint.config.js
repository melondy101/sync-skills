// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Mechanical gate for the conventions CONTRIBUTING.md mandates:
// 1. Frontend components must not call invoke directly — all Tauri calls go
//    through src/api.ts, so importing "@tauri-apps/api/core" is only allowed there.
// 2. Every source file keeps the copyright + SPDX header at the top.
import { defineConfig, globalIgnores } from "eslint/config";
import tsParser from "@typescript-eslint/parser";
import reactHooks from "eslint-plugin-react-hooks";

// Local rule: the file must start with a comment block containing the
// SPDX-License-Identifier declaration (see CONTRIBUTING.md 代码规范/通用).
const spdxHeaderPlugin = {
  rules: {
    "require-spdx-header": {
      meta: {
        type: "problem",
        docs: {
          description:
            "require the copyright + SPDX header comment at the top of each source file",
        },
        messages: {
          missing:
            "Missing SPDX header. Start the file with:\n// Copyright (c) 2026 Skill Manager Contributors\n// SPDX-License-Identifier: AGPL-3.0-only",
        },
        schema: [],
      },
      create(context) {
        return {
          Program(node) {
            const header = context.sourceCode
              .getAllComments()
              .filter((comment) => comment.loc.start.line <= 5)
              .map((comment) => comment.value)
              .join("\n");
            if (!/SPDX-License-Identifier:\s*AGPL-3\.0-only/.test(header)) {
              context.report({
                loc: { line: 1, column: 0 },
                messageId: "missing",
              });
            }
          },
        };
      },
    },
  },
};

export default defineConfig([
  globalIgnores(["dist/", "src-tauri/", ".qoder/", ".learn-tmp/", "doc/", "docs/"]),
  {
    files: ["src/**/*.{ts,tsx}", "*.config.ts"],
    languageOptions: {
      parser: tsParser,
    },
    plugins: {
      "skill-manager": spdxHeaderPlugin,
      "react-hooks": reactHooks,
    },
    rules: {
      "skill-manager/require-spdx-header": "error",
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "@tauri-apps/api/core",
              message:
                "Frontend code must not call invoke directly; add a wrapper in src/api.ts instead (see CONTRIBUTING.md).",
            },
          ],
        },
      ],
    },
  },
  {
    // src/api.ts is the single allowed owner of the invoke bridge.
    files: ["src/api.ts"],
    rules: {
      "no-restricted-imports": "off",
    },
  },
]);
