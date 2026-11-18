import { useIsMutating, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { analyze, type Progress } from './client';

export const queryKeys = {
  setup: ['setup'] as const,
  playsets: ['playsets'] as const,
  summary: ['summary'] as const,
  analyze: ['analyze'] as const,
  analysisProgress: ['analysisProgress'] as const,
  pair: (winnerId: string | null, loserId: string | null) => ['pair', winnerId, loserId] as const,
  search: (query: unknown) => ['search', query] as const,
  rules: ['rules'] as const,
  proposal: ['proposal'] as const,
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
      for (const key of ['pair', 'search', 'rules', 'proposal', 'calibration']) {
        queryClient.removeQueries({ queryKey: [key] });
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
