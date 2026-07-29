// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback, useState } from "react";
import type { Toast } from "../types";

export type AddToastFn = (type: Toast["type"], message: string) => void;

/** Toast queue with auto-expiry (4s). */
export function useToasts() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const addToast = useCallback<AddToastFn>((type, message) => {
    const id = Date.now() + Math.random();
    setToasts((prev) => [...prev, { type, message, id }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 4000);
  }, []);

  return { toasts, addToast };
}
