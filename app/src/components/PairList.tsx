import { Box, Group, Text, UnstyledButton } from '@mantine/core';
import { IconArrowRight, IconChevronDown, IconChevronRight } from '@tabler/icons-react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { PairView } from '../api/bindings/PairView';
import { pairConflicts } from '../api/client';
import { queryKeys } from '../api/queries';
import { sourceName } from '../explain';
import { ConflictList } from './ConflictList';
import { VerdictBadge } from './ConflictItem';
import { MarkIntendedButton } from './MarkIntendedButton';

const pairKey = (p: PairView) => JSON.stringify([p.winner.id, p.loser.id]);

/** Pairs «winner → loser»; the open pair shows its conflicts under its row. */
export function PairList({
  pairs,
  openKey,
  onToggle,
}: {
  pairs: PairView[];
  openKey: string | null;
  onToggle: (key: string | null) => void;
}) {
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line react-hooks/incompatible-library -- the app is not built with React Compiler
  const virtualizer = useVirtualizer({
    count: pairs.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 40,
    overscan: 5,
    getItemKey: (i) => pairKey(pairs[i]),
  });

  if (pairs.length === 0) {
    return (
      <Text size="sm" c="dimmed">
        {t('conflicts.noPairs')}
      </Text>
    );
  }

  return (
    <div ref={scrollRef} style={{ maxHeight: 'calc(100vh - 240px)', overflowY: 'auto' }}>
      <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
        {virtualizer.getVirtualItems().map((row) => {
          const pair = pairs[row.index];
          const key = pairKey(pair);
          const open = key === openKey;
          const winner = sourceName(t, pair.winner);
          const loser = sourceName(t, pair.loser);
          const counts = [
            pair.files > 0 && t('counts.files', { count: pair.files }),
            pair.objects > 0 && t('counts.objects', { count: pair.objects }),
            pair.loc > 0 && t('counts.loc', { count: pair.loc }),
          ].filter((c) => c !== false);
          return (
            <div
              key={row.key}
              ref={virtualizer.measureElement}
              data-index={row.index}
              data-pair
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${row.start}px)`,
                borderBottom: '1px solid var(--mantine-color-default-border)',
              }}
            >
              <Group gap="sm" wrap="nowrap" px="xs" py={6}>
                <UnstyledButton
                  aria-expanded={open}
                  aria-label={t('conflicts.pair', { winner, loser })}
                  onClick={() => onToggle(open ? null : key)}
                  style={{ flex: 1, minWidth: 0 }}
                >
                  <Group gap={6} wrap="nowrap">
                    {open ? (
                      <IconChevronDown size={14} style={{ flexShrink: 0 }} />
                    ) : (
                      <IconChevronRight size={14} style={{ flexShrink: 0 }} />
                    )}
                    <Text size="sm" fw={500} truncate>
                      {winner}
                    </Text>
                    <IconArrowRight size={14} style={{ flexShrink: 0 }} />
                    <Text size="sm" truncate>
                      {loser}
                    </Text>
                  </Group>
                </UnstyledButton>
                {counts.map((c) => (
                  <Text key={c} size="xs" c="dimmed" style={{ whiteSpace: 'nowrap' }}>
                    {c}
                  </Text>
                ))}
                <VerdictBadge verdict={pair.verdict} />
                <MarkIntendedButton pair={pair} />
              </Group>
              {open && (
                <Box pl="lg" pr="xs" pb="sm">
                  <ConflictList
                    queryKey={queryKeys.pair(pair.winner.id, pair.loser.id)}
                    fetchPage={(offset, limit) =>
                      pairConflicts(pair.winner.id, pair.loser.id, offset, limit)
                    }
                    maxHeight="40vh"
                  />
                </Box>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
