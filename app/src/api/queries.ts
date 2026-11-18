import {
  useIsMutating,
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
} from '@tanstack/react-query';
import { analyze, setVanilla, summary, type Progress } from './client';

export const queryKeys = {
  setup: ['setup'] as const,
  playsets: ['playsets'] as const,
  summary: ['summary'] as const,
  analyze: ['analyze'] as const,
  analysisProgress: ['analysisProgress'] as const,
  analysisError: ['analysisError'] as const,
  pair: (winnerId: string | null, loserId: string | null) => ['pair', winnerId, loserId] as const,
  search: (query: unknown) => ['search', query] as const,
  rules: ['rules'] as const,
  proposal: ['proposal'] as const,
  applied: ['applied'] as const,
  launcher: ['launcher'] as const,
  calibration: ['calibration'] as const,
};

export function useAnalyze() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationKey: queryKeys.analyze,
    mutationFn: ({ playsetId }: { playsetId: string | null }) =>
      analyze(playsetId, (p) => queryClient.setQueryData(queryKeys.analysisProgress, p)),
    onSuccess: (data) => {
      queryClient.setQueryData(queryKeys.summary, data);
      queryClient.setQueryData(queryKeys.analysisError, null);
      queryClient.setQueryData(queryKeys.applied, null);
      for (const key of ['pair', 'search', 'rules', 'proposal', 'calibration']) {
        queryClient.removeQueries({ queryKey: [key] });
      }
    },
    onError: (error) => {
      // Rust keeps the previous analysis after a failed analyze, so `summary` stays.
      queryClient.setQueryData(queryKeys.analysisError, error);
    },
    onSettled: () => queryClient.removeQueries({ queryKey: queryKeys.analysisProgress }),
  });
}

/**
 * `set_vanilla` recomputes the report (and may index the game), so it shares the analysis key
 * prefix and progress: the shell shows its progress and every analysis action waits for it.
 */
export function useSetVanilla() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationKey: [...queryKeys.analyze, 'vanilla'],
    mutationFn: (on: boolean) =>
      setVanilla(on, (p) => queryClient.setQueryData(queryKeys.analysisProgress, p)),
    onSuccess: (data) => {
      queryClient.setQueryData(queryKeys.summary, data);
      for (const key of ['pair', 'search']) void queryClient.resetQueries({ queryKey: [key] });
      for (const key of ['rules', 'proposal', 'calibration', 'setup']) {
        void queryClient.invalidateQueries({ queryKey: [key] });
      }
    },
    onSettled: () => queryClient.removeQueries({ queryKey: queryKeys.analysisProgress }),
  });
}

export function useAnalysisRunning(): { running: boolean; progress: Progress | null } {
  const running = useIsMutating({ mutationKey: queryKeys.analyze }) > 0;
  const progress = useQuery<Progress | null>({
    queryKey: queryKeys.analysisProgress,
    queryFn: () => null,
    enabled: false,
  });
  return { running, progress: progress.data ?? null };
}

export function useAnalysisError(): unknown {
  return useQuery<unknown>({
    queryKey: queryKeys.analysisError,
    queryFn: () => null,
    enabled: false,
  }).data;
}

/**
 * After `rules.toml` changed: the analysis of the Rust side is recomputed (ruling R16). The edit
 * already succeeded, so a failing `summary()` must not turn it into an error; the summary is only
 * refreshed when there is one, and the next analysis replaces a stale one.
 */
export async function refreshAfterRuleEdit(queryClient: QueryClient): Promise<void> {
  for (const key of ['pair', 'search']) void queryClient.resetQueries({ queryKey: [key] });
  void queryClient.invalidateQueries({ queryKey: queryKeys.proposal });
  if (queryClient.getQueryData(queryKeys.summary) === undefined) return;
  try {
    queryClient.setQueryData(queryKeys.summary, await summary());
  } catch {
    // the rule edit is already saved; keep the previous summary
  }
}
