'use client';

import { useTranslation } from '@/context/language-context';

export function Footer() {
    const { t } = useTranslation();

    return (
        <footer className="fixed bottom-0 left-0 right-0 h-10 z-50 flex items-center justify-center border-t bg-background text-sm text-muted-foreground">
            © {new Date().getFullYear()} Learning By Machine. {t('footer')}
        </footer>
    );
}