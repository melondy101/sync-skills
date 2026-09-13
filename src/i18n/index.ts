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