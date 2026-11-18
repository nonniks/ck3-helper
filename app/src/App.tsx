import { AppShell, MantineProvider, NavLink } from '@mantine/core';
import { Notifications } from '@mantine/notifications';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

const queryClient = new QueryClient();
const screens = ['playset', 'conflicts', 'order', 'calibration', 'settings'] as const;

export default function App() {
  const { t } = useTranslation();
  const [active, setActive] = useState<(typeof screens)[number]>('playset');
  return (
    <MantineProvider defaultColorScheme="auto">
      <QueryClientProvider client={queryClient}>
        <Notifications />
        <AppShell navbar={{ width: 220, breakpoint: 0 }} padding="md">
          <AppShell.Navbar p="xs">
            {screens.map((screen) => (
              <NavLink
                key={screen}
                label={t(`nav.${screen}`)}
                active={active === screen}
                onClick={() => setActive(screen)}
              />
            ))}
          </AppShell.Navbar>
          <AppShell.Main />
        </AppShell>
      </QueryClientProvider>
    </MantineProvider>
  );
}
