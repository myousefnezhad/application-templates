// any client component
"use client"
import useAppStore from "@/store/useAppStore"

export default function Header() {
    const { theme, setTheme } = useAppStore()
    return <button onClick={() => setTheme("dark")}>{theme}</button>
}