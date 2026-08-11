// Copyright (c) 2026 Skill Manager Contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import ConfirmDialog, { type ConfirmVariant, type SyncProgress } from "./ConfirmDialog";

export type ConfirmRequest = {
  title: string;
  message: string;
  detail?: string;
  confirmText: string;
  cancelText: string;
  confirmVariant?: ConfirmVariant;
  /** Async action to run when the user confirms. Errors bubble via the caller's own toast. */
  onConfirm: () => Promise<void> | void;
  /** Optional progress payload. Components push updates via `setProgress` from the hook. */
  progress?: SyncProgress | null;
};

type ConfirmContextValue = {
  /** Open a confirmation dialog. The dialog stays open until the user confirms, cancels, or the request is overwritten. */
  showConfirm: (req: ConfirmRequest) => void;
  /** Update the progress payload of the currently-open dialog. No-op if no dialog is open. */
  setProgress: (progress: SyncProgress | null) => void;
};

const ConfirmContext = createContext<ConfirmContextValue | null>(null);

export function useConfirm(): ConfirmContextValue {
  const ctx = useContext(ConfirmContext);
  if (!ctx) {
    throw new Error("useConfirm must be used inside <ConfirmProvider>");
  }
  return ctx;
}

type ProviderProps = {
  children: ReactNode;
};

export function ConfirmProvider({ children }: ProviderProps) {
  const [request, setRequest] = useState<ConfirmRequest | null>(null);
  const [running, setRunning] = useState(false);

  const showConfirm = useCallback((req: ConfirmRequest) => {
    setRequest({ confirmVariant: "primary", ...req, progress: req.progress ?? null });
  }, []);

  const setProgress = useCallback((progress: SyncProgress | null) => {
    setRequest((prev) => (prev ? { ...prev, progress } : prev));
  }, []);

  const handleConfirm = useCallback(async () => {
    if (!request) return;
    const action = request.onConfirm;
    setRunning(true);
    void Promise.resolve(action()).finally(() => {
      setRunning(false);
      setRequest(null);
    });
  }, [request]);

  const handleCancel = useCallback(() => {
    setRequest((prev) => (prev?.progress ? prev : null));
  }, []);

  const value = useMemo<ConfirmContextValue>(() => ({ showConfirm, setProgress }), [showConfirm, setProgress]);

  return (
    <ConfirmContext.Provider value={value}>
      {children}
      {request && (
        <ConfirmDialog
          title={request.title}
          message={request.message}
          detail={request.detail}
          confirmText={request.confirmText}
          cancelText={request.cancelText}
          confirmVariant={request.confirmVariant}
          progress={request.progress}
          loading={running}
          onConfirm={handleConfirm}
          onCancel={handleCancel}
        />
      )}
    </ConfirmContext.Provider>
  );
}
