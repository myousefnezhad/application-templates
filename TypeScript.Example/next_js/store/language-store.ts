import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type Language = 'en' | 'fr';

interface LanguageState {
    language: Language;
    setLanguage: (lang: Language) => void;
    toggleLanguage: () => void;
}

export const useLanguageStore = create<LanguageState>()(
    persist(
        (set) => ({
            language: 'en',
            setLanguage: (language) => set({ language }),
            toggleLanguage: () =>
                set((state) => ({ language: state.language === 'en' ? 'fr' : 'en' })),
        }),
        { name: 'language-storage', skipHydration: true }
    )
);