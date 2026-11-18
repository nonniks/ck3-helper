import { Badge, Group, Table, Text } from '@mantine/core';
import { useTranslation } from 'react-i18next';
import type { MoveView } from '../api/bindings/MoveView';
import type { OrderRow } from '../api/bindings/OrderRow';
import { MoveReasons } from './MoveReasons';

export function OrderTable({ rows, moves }: { rows: OrderRow[]; moves: MoveView[] }) {
  const { t } = useTranslation();
  const reasonsById = new Map(moves.map((m) => [m.id, m.reasons]));
  return (
    <Table striped>
      <Table.Thead>
        <Table.Tr>
          <Table.Th>{t('order.colNew')}</Table.Th>
          <Table.Th>{t('order.colName')}</Table.Th>
          <Table.Th>{t('order.colWas')}</Table.Th>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>
        {rows.map((row) => (
          <Table.Tr key={row.id} c={row.enabled ? undefined : 'dimmed'}>
            <Table.Td>{row.to + 1}</Table.Td>
            <Table.Td>
              <Group gap="xs">
                <Text size="sm" fw={row.moved ? 600 : undefined}>
                  {row.name}
                </Text>
                {!row.enabled && (
                  <Badge color="gray" variant="light">
                    {t('order.disabled')}
                  </Badge>
                )}
              </Group>
              {row.moved && <MoveReasons reasons={reasonsById.get(row.id) ?? []} />}
            </Table.Td>
            <Table.Td>{row.moved ? t('order.was', { from: row.from + 1 }) : null}</Table.Td>
          </Table.Tr>
        ))}
      </Table.Tbody>
    </Table>
  );
}
