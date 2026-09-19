// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

// PRD §11 "定时检测同步" as an in-app schedule. It lives in the window rather
// than in a spawned job because PRD §10 rules out background processes: closing
// the app has to stop the timer.

import { useEffect, useRef } from "react";

/**
 * Run `onTick` every `intervalMinutes`; `0` (or less) turns the schedule off and
 * installs no timer at all. The callback is read through a ref so the caller can
 * pass a fresh closure each render without the interval restarting.
 */
export function useUpdateSchedule(intervalMinutes: number, onTick: () => void): void {
  const tickRef = useRef(onTick);

  useEffect(() => {
    tickRef.current = onTick;
  });

  useEffect(() => {
    if (!(intervalMinutes > 0)) return;
    const timer = window.setInterval(() => tickRef.current(), intervalMinutes * 60_000);
    return () => window.clearInterval(timer);
  }, [intervalMinutes]);
}
