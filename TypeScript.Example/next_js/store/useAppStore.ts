// store/useAppStore.ts
import { create } from "zustand"

const useAppStore = create((set) => ({
    lang: "en",
    theme: "light",
    setLang: (lang) => set({ lang }),
    setTheme: (theme) => set({ theme }),
}))

export default useAppStore