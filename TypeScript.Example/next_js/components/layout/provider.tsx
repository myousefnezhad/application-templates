'use client';

import { ThemeProvider, useTheme } from '@/context/theme-context';
import { LanguageProvider, useLanguage } from '@/context/language-context';
import { AuthProvider } from '@/context/auth-context';

function ReadyGate({ children }: { children: React.ReactNode }) {
    const { isReady: themeReady } = useTheme();
    const { isReady: langReady } = useLanguage();

    if (!themeReady || !langReady) return null; // or a loading skeleton

    return <>{children}</>;
}

export function Providers({ children }: { children: React.ReactNode }) {
    return (
        <ThemeProvider>
            <LanguageProvider>
                <ReadyGate>
                    <AuthProvider>{children}</AuthProvider>
                </ReadyGate>
            </LanguageProvider>
        </ThemeProvider>
    );
}