import type { Language } from '@/store/language-store';

export const translations = {
    en: {
        dashboard: 'Dashboard',
        login: 'Log in',
        logout: 'Log out',
        footer: 'All rights reserved.',
        darkMode: 'Dark',
        lightMode: 'Light',
    },
    fr: {
        dashboard: 'Tableau de bord',
        login: 'Connexion',
        logout: 'Déconnexion',
        footer: 'Tous droits réservés.',
        darkMode: 'Sombre',
        lightMode: 'Clair',
    },
} satisfies Record<Language, Record<string, string>>;

export type TranslationKey = keyof typeof translations.en;