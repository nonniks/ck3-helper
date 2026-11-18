import { useMutation, useQueryClient } from '@tanstack/react-query';
import { analyze, type Progress } from './client';

export const queryKeys = {
  setup: ['setup'] as const,
  playsets: ['playsets'] as const,
  summary: ['summary'] as const,
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
    mutationFn: ({
      playsetId,
      onProgress,
    }: {
      playsetId: string | null;
      onProgress: (p: Progress) => void;
    }) => analyze(playsetId, onProgress),
    onSuccess: (data) => {
      queryClient.setQueryData(queryKeys.summary, data);
      for (const key of ['pair', 'search', 'rules', 'proposal', 'calibration']) {
        queryClient.removeQueries({ queryKey: [key] });
      }
    },
  });
}
