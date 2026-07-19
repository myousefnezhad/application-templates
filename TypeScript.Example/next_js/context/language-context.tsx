'use client';

import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { useLanguageStore, Language } from '@/store/language-store';
import { translations, TranslationKey } from '@/lib/translations';

interface LanguageContextType {
    language: Language;
    setLanguage: (lang: Language) => void;
    toggleLanguage: () => void;
    isReady: boolean;
}

const LanguageContext = createContext<LanguageContextType | undefined>(undefined);

export function LanguageProvider({ children }: { children: ReactNode }) {
    const [isReady, setIsReady] = useState(false);

    const language = useLanguageStore((s) => s.language);
    const setLanguage = useLanguageStore((s) => s.setLanguage);
    const toggleLanguage = useLanguageStore((s) => s.toggleLanguage);

    useEffect(() => {
        useLanguageStore.persist.rehydrate();
        setIsReady(true);
    }, []);

    return (
        <LanguageContext.Provider value={{ language, setLanguage, toggleLanguage, isReady }}>
            {children}
        </LanguageContext.Provider>
    );
}

export function useLanguage() {
    const context = useContext(LanguageContext);
    if (context === undefined) {
        throw new Error('useLanguage must be used within a LanguageProvider');
    }
    return context;
}

export function useTranslation() {
    const { language } = useLanguage();
    const t = (key: TranslationKey) => translations[language][key];
    return { t, language };
}