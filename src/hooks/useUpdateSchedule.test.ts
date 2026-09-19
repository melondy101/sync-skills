// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook } from "@testing-library/react";
import { useUpdateSchedule } from "./useUpdateSchedule";

describe("useUpdateSchedule", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("ticks on the configured interval and keeps ticking", () => {
    const tick = vi.fn();
    renderHook(() => useUpdateSchedule(10, tick));

    vi.advanceTimersByTime(9 * 60_000);
    expect(tick).not.toHaveBeenCalled();

    vi.advanceTimersByTime(60_000);
    expect(tick).toHaveBeenCalledTimes(1);

    vi.advanceTimersByTime(10 * 60_000);
    expect(tick).toHaveBeenCalledTimes(2);
  });

  it("installs no timer at all when the schedule is off", () => {
    const spy = vi.spyOn(window, "setInterval");
    const tick = vi.fn();
    renderHook(() => useUpdateSchedule(0, tick));

    vi.advanceTimersByTime(6 * 60 * 60_000);

    expect(spy).not.toHaveBeenCalled();
    expect(tick).not.toHaveBeenCalled();
    spy.mockRestore();
  });

  it("reads the newest callback without restarting the countdown", () => {
    const first = vi.fn();
    const second = vi.fn();
    const { rerender } = renderHook(
      ({ minutes, tick }: { minutes: number; tick: () => void }) => useUpdateSchedule(minutes, tick),
      { initialProps: { minutes: 10, tick: first } }
    );

    rerender({ minutes: 10, tick: second });
    vi.advanceTimersByTime(10 * 60_000);

    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);
  });

  it("clears the interval on unmount and on turning the schedule off", () => {
    const tick = vi.fn();
    const { unmount } = renderHook(() => useUpdateSchedule(10, tick));
    unmount();
    vi.advanceTimersByTime(60 * 60_000);
    expect(tick).not.toHaveBeenCalled();

    const spy = vi.spyOn(window, "clearInterval");
    const { rerender } = renderHook(
      ({ minutes }: { minutes: number }) => useUpdateSchedule(minutes, tick),
      { initialProps: { minutes: 10 } }
    );
    rerender({ minutes: 0 });
    expect(spy).toHaveBeenCalled();
    spy.mockRestore();
  });
});
