// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import zh from "./zh";
import en from "./en";

export type Lang = "zh" | "en";

export type TranslateFn = (key: string) => string;

export const translations: Record<Lang, Record<string, string>> = {
  zh,
  en,
};

export function makeT(lang: Lang): TranslateFn {
  return (key: string) => translations[lang]?.[key] || translations.en[key] || key;
}

/**
 * Path-validation failures come back from the backend as stable ASCII codes with
 * the offending input appended (see `src-tauri/src/paths.rs`), so the message can
 * be localized without parsing prose. Anything unrecognized passes through
 * untouched — a raw backend error is still more useful than a generic one.
 */
export function localizeApiError(t: TranslateFn, message: string): string {
  const separator = message.indexOf(":");
  const code = separator < 0 ? message : message.slice(0, separator);
  const detail = separator < 0 ? "" : message.slice(separator + 1);
  switch (code) {
    case "empty-path":
      return t("pathErrorEmpty");
    case "env-var-unsupported":
      return `${t("pathErrorEnvVar")}: ${detail}`;
    case "relative-path":
      return `${t("pathErrorRelative")}: ${detail}`;
    case "path-not-found":
      return `${t("pathErrorNotFound")}: ${detail}`;
    default:
      return message;
  }
}