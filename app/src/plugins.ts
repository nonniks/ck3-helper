import { notifications } from '@mantine/notifications';
import { i18n } from './i18n';

export async function pluginCall<T>(call: () => Promise<T>): Promise<T | undefined> {
  try {
    return await call();
  } catch (e) {
    notifications.show({
      color: 'red',
      message: i18n.t('pluginError', { detail: e instanceof Error ? e.message : String(e) }),
    });
    return undefined;
  }
}
