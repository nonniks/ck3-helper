import { Channel, invoke } from '@tauri-apps/api/core';
import type { AnalysisView } from './bindings/AnalysisView';
import type { AppError } from './bindings/AppError';
import type { ApplyPreviewView } from './bindings/ApplyPreviewView';
import type { ApplyView } from './bindings/ApplyView';
import type { CalibrationView } from './bindings/CalibrationView';
import type { ConflictQuery } from './bindings/ConflictQuery';
import type { ConflictView } from './bindings/ConflictView';
import type { IndexWarningView } from './bindings/IndexWarningView';
import type { LauncherStatus } from './bindings/LauncherStatus';
import type { Page } from './bindings/Page';
import type { PlaysetRow } from './bindings/PlaysetRow';
import type { Progress } from './bindings/Progress';
import type { ProposalView } from './bindings/ProposalView';
import type { RulesView } from './bindings/RulesView';
import type { Settings } from './bindings/Settings';
import type { SetupView } from './bindings/SetupView';
import type { UpdateView } from './bindings/UpdateView';

export type { AppError, Progress };

export function isAppError(e: unknown): e is AppError {
  return (
    typeof e === 'object' &&
    e !== null &&
    typeof (e as AppError).code === 'string' &&
    Array.isArray((e as AppError).items)
  );
}

function channel(onProgress: (p: Progress) => void): Channel<Progress> {
  const c = new Channel<Progress>();
  c.onmessage = onProgress;
  return c;
}

export const getSetup = () => invoke<SetupView>('get_setup');
export const saveSettings = (settings: Settings) =>
  invoke<SetupView>('save_settings', { settings });
export const listPlaysets = () => invoke<PlaysetRow[]>('list_playsets');
export const analyze = (playsetId: string | null, onProgress: (p: Progress) => void) =>
  invoke<AnalysisView>('analyze', { playsetId, onProgress: channel(onProgress) });
export const setVanilla = (on: boolean, onProgress: (p: Progress) => void) =>
  invoke<AnalysisView>('set_vanilla', { on, onProgress: channel(onProgress) });
export const summary = () => invoke<AnalysisView>('summary');
export const pairConflicts = (
  winnerId: string | null,
  loserId: string | null,
  offset: number,
  limit: number,
) => invoke<Page<ConflictView>>('pair_conflicts', { winnerId, loserId, offset, limit });
export const searchConflicts = (query: ConflictQuery, offset: number, limit: number) =>
  invoke<Page<ConflictView>>('search_conflicts', { query, offset, limit });
export const modWarnings = (modId: string) => invoke<IndexWarningView[]>('mod_warnings', { modId });
export const rules = () => invoke<RulesView>('rules');
export const addRule = (before: string, after: string, note: string | null) =>
  invoke<RulesView>('add_rule', { before, after, note });
export const markIntended = (winnerId: string, loserId: string, note: string) =>
  invoke<RulesView>('mark_intended', { winnerId, loserId, note });
export const removeRule = (index: number) => invoke<RulesView>('remove_rule', { index });
export const propose = () => invoke<ProposalView>('propose');
export const launcherStatus = () => invoke<LauncherStatus>('launcher_status');
export const applyPreview = () => invoke<ApplyPreviewView>('apply_preview');
export const apply = () => invoke<ApplyView>('apply');
export const calibrate = (onProgress: (p: Progress) => void) =>
  invoke<CalibrationView>('calibrate', { onProgress: channel(onProgress) });
export const checkUpdate = () => invoke<UpdateView | null>('check_update');
