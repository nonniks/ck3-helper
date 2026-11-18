import { Alert, Loader, Paper, Stack, Text } from '@mantine/core';
import { useInfiniteQuery } from '@tanstack/react-query';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { ConflictView } from '../api/bindings/ConflictView';
import type { Page } from '../api/bindings/Page';
import { errorMessage } from '../errors';
import { ConflictItem } from './ConflictItem';

const PAGE_SIZE = 100;
/** The next page is requested once the last rendered row is this close to the loaded end. */
const PREFETCH_ROWS = 20;

/** Conflicts loaded page by page as the list scrolls; only the rows in view are rendered. */
export function ConflictList({
  queryKey,
  fetchPage,
  maxHeight,
}: {
  queryKey: readonly unknown[];
  fetchPage: (offset: number, limit: number) => Promise<Page<ConflictView>>;
  maxHeight: string;
}) {
  const { t } = useTranslation();
  const query = useInfiniteQuery({
    queryKey,
    queryFn: ({ pageParam }) => fetchPage(pageParam, PAGE_SIZE),
    initialPageParam: 0,
    getNextPageParam: (last) => {
      const next = last.offset + last.items.length;
      return last.items.length > 0 && next < last.total ? next : undefined;
    },
  });
  const items = useMemo(() => query.data?.pages.flatMap((p) => p.items) ?? [], [query.data]);
  const total = query.data?.pages.at(-1)?.total ?? 0;

  const scrollRef = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line react-hooks/incompatible-library -- the app is not built with React Compiler
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 120,
    overscan: 5,
    gap: 8,
  });
  const rows = virtualizer.getVirtualItems();
  const lastRendered = rows.at(-1)?.index ?? -1;

  const { hasNextPage, isFetchingNextPage, isFetchNextPageError, fetchNextPage } = query;
  useEffect(() => {
    if (
      hasNextPage &&
      !isFetchingNextPage &&
      !isFetchNextPageError &&
      lastRendered >= items.length - PREFETCH_ROWS
    ) {
      void fetchNextPage();
    }
  }, [
    hasNextPage,
    isFetchingNextPage,
    isFetchNextPageError,
    lastRendered,
    items.length,
    fetchNextPage,
  ]);

  if (query.isPending) return <Loader size="sm" />;
  if (query.isError && !query.data) {
    return <Alert color="red">{errorMessage(t, query.error)}</Alert>;
  }
  if (total === 0) {
    return (
      <Text size="sm" c="dimmed">
        {t('conflicts.noConflicts')}
      </Text>
    );
  }

  return (
    <Stack gap="xs">
      <Text size="xs" c="dimmed">
        {t('conflicts.total', { count: total })}
      </Text>
      <div ref={scrollRef} data-conflict-list style={{ maxHeight, overflowY: 'auto' }}>
        <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
          {rows.map((row) => (
            <div
              key={row.key}
              ref={virtualizer.measureElement}
              data-index={row.index}
              data-testid="conflict-row"
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${row.start}px)`,
              }}
            >
              <Paper withBorder p="xs">
                <ConflictItem conflict={items[row.index]} />
              </Paper>
            </div>
          ))}
        </div>
      </div>
      {isFetchNextPageError && <Alert color="red">{errorMessage(t, query.error)}</Alert>}
    </Stack>
  );
}
