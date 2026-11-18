import { Badge, Button, Group, Table, Text } from '@mantine/core';
import { IconFolderOpen } from '@tabler/icons-react';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import { pluginCall } from '../plugins';
import { useTranslation } from 'react-i18next';
import type { ModView } from '../api/bindings/ModView';

export function ModTable({
  mods,
  onShowWarnings,
}: {
  mods: ModView[];
  onShowWarnings: (mod: ModView) => void;
}) {
  const { t } = useTranslation();
  return (
    <Table striped highlightOnHover>
      <Table.Thead>
        <Table.Tr>
          <Table.Th>{t('playset.colPosition')}</Table.Th>
          <Table.Th>{t('playset.colName')}</Table.Th>
          <Table.Th>{t('playset.colStatus')}</Table.Th>
          <Table.Th>{t('playset.colActions')}</Table.Th>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>
        {mods.map((mod) => (
          <Table.Tr key={mod.id} c={mod.enabled ? undefined : 'dimmed'}>
            <Table.Td>{mod.position + 1}</Table.Td>
            <Table.Td>
              <Text size="sm">{mod.name}</Text>
              {mod.version && (
                <Text size="xs" c="dimmed">
                  {mod.version}
                </Text>
              )}
            </Table.Td>
            <Table.Td>
              <Group gap={4}>
                {!mod.enabled && (
                  <Badge color="gray" variant="light">
                    {t('playset.disabled')}
                  </Badge>
                )}
                {mod.missing && (
                  <Badge color="red" variant="light">
                    {t('playset.missing')}
                  </Badge>
                )}
                {mod.depIssues.map((d) => (
                  <Badge key={d.dependency} color="orange" variant="light">
                    {t(d.ambiguous ? 'playset.depAmbiguous' : 'playset.depMissing', {
                      name: d.dependency,
                    })}
                  </Badge>
                ))}
                {mod.compatch && (
                  <Badge color="blue" variant="light">
                    {t('playset.compatch')}
                  </Badge>
                )}
                {mod.replacePaths.map((p) => (
                  <Badge key={p} color="grape" variant="light">
                    {t('playset.replacePath', { path: p })}
                  </Badge>
                ))}
              </Group>
            </Table.Td>
            <Table.Td>
              <Group gap="xs" wrap="nowrap">
                {mod.warnings > 0 && (
                  <Button
                    size="compact-xs"
                    variant="light"
                    color="yellow"
                    data-warnings
                    aria-label={`${t('playset.warnings', { count: mod.warnings })} — ${mod.name}`}
                    onClick={() => onShowWarnings(mod)}
                  >
                    {mod.warnings}
                  </Button>
                )}
                <Button
                  size="compact-xs"
                  variant="subtle"
                  data-open
                  disabled={mod.missing}
                  aria-label={`${t('playset.openFolder')} — ${mod.name}`}
                  leftSection={<IconFolderOpen size={14} />}
                  onClick={() => void pluginCall(() => revealItemInDir(mod.dir))}
                >
                  {t('playset.openFolder')}
                </Button>
              </Group>
            </Table.Td>
          </Table.Tr>
        ))}
      </Table.Tbody>
    </Table>
  );
}
