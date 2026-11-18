import { Badge, Group, Stack, Text } from '@mantine/core';
import { useTranslation } from 'react-i18next';
import type { ConflictView } from '../api/bindings/ConflictView';
import type { ContenderView } from '../api/bindings/ContenderView';
import type { VerdictView } from '../api/bindings/VerdictView';
import { explain, sourceName } from '../explain';

const verdictColor: Record<VerdictView, string> = {
  suspicious: 'orange',
  intended: 'green',
  vanilla: 'gray',
};

export function VerdictBadge({ verdict }: { verdict: VerdictView }) {
  const { t } = useTranslation();
  return (
    <Badge size="sm" variant="light" color={verdictColor[verdict]}>
      {t(`verdict.${verdict}`)}
    </Badge>
  );
}

function Contender({ label, contender }: { label: string; contender: ContenderView }) {
  const { t } = useTranslation();
  return (
    <div>
      <Group gap={6} wrap="nowrap">
        <Text size="xs" c="dimmed">
          {label}
        </Text>
        <Text size="sm" fw={500}>
          {sourceName(t, contender.source)}
        </Text>
        <Text size="xs" c="dimmed" ff="monospace" truncate>
          {contender.file}
        </Text>
      </Group>
      {contender.text !== null && (
        <Text size="sm" pl="md">
          {contender.text}
        </Text>
      )}
    </div>
  );
}

export function ConflictItem({ conflict }: { conflict: ConflictView }) {
  const { t } = useTranslation();
  return (
    <Stack gap={4}>
      <Group gap="xs" wrap="nowrap">
        <Badge size="sm" variant="outline" color="gray">
          {t(`level.${conflict.level}`)}
        </Badge>
        <Text size="sm" fw={600} ff="monospace" style={{ wordBreak: 'break-all' }}>
          {conflict.key}
        </Text>
        {conflict.database && (
          <Text size="xs" c="dimmed" ff="monospace">
            {conflict.database}
          </Text>
        )}
        <VerdictBadge verdict={conflict.verdict} />
      </Group>
      <Contender label={t('conflicts.winner')} contender={conflict.winner} />
      {conflict.losers.map((loser, i) => (
        <Contender key={i} label={t('conflicts.loser')} contender={loser} />
      ))}
      {explain(t, conflict).map((line) => (
        <Text key={line} size="xs" c="dimmed">
          {line}
        </Text>
      ))}
    </Stack>
  );
}
