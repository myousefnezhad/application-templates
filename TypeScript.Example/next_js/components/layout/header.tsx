'use client';

import { useAuth } from '@/context/auth-context';
import { useTheme } from '@/context/theme-context';
import { useLanguage, useTranslation } from '@/context/language-context';

export function Header() {
    const { theme, toggleTheme } = useTheme();
    const { language, setLanguage } = useLanguage();
    const { t } = useTranslation();
    const { isLogin, logout } = useAuth();

    return (
        <header className="fixed top-0 left-0 right-0 h-16 z-50 flex items-center justify-between px-6 border-b border-border bg-background">
            <div className="flex items-center gap-2 font-semibold text-lg">
                <span className="text-xl">🔷</span>
                <span>{isLogin ? t('dashboard') : t('login')}</span>
            </div>

            <div className="flex items-center gap-4">
                <button
                    type="button"
                    onClick={() => {language === 'en' ? setLanguage('fr') : setLanguage('en')}}
                    className="border border-border rounded px-3 py-1 text-sm"
                >
                    {language === 'en' ? `FR` : `EN`}
                </button>
                <button
                    type="button"
                    onClick={toggleTheme}
                    className="border border-border rounded px-3 py-1 text-sm"
                >
                    {theme === 'light' ? `🌙` : `☀️`}
                </button>
                {isLogin && (
                    <button
                        type="button"
                        onClick={logout}
                        className="border border-border rounded px-3 py-1 text-sm"
                    >
                        {`🔒`}
                    </button>
                )}
            </div>
        </header>
    );
}