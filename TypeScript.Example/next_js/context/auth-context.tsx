'use client';

import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { jsonCall } from '@/lib/net/api';

const STORAGE_KEY = 'token';

interface AuthContextType {
    isLogin: boolean;
    isLoading: boolean;
    login: (email: string, password: string) => Promise<string | null>;
    logout: () => Promise<string | null>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
    const [isLogin, setIsLogin] = useState(false);
    const [isLoading, setIsLoading] = useState(true);

    // Rehydrate from localStorage on mount (client only)
    useEffect(() => {
        try {
            const checkLoginStatus = async () => {
                const stored = localStorage.getItem(STORAGE_KEY);
                const {res, error} = await jsonCall('/auth/ping', 'POST');
                if (error || !res) {
                    setIsLogin(false);
                    setIsLoading(false);
                    localStorage.removeItem(STORAGE_KEY);
                    return;
                } 



                setIsLogin(!!stored);
                setIsLoading(false);
            };
            checkLoginStatus();
        } catch {
            setIsLoading(false);            
            setIsLogin(false);
            localStorage.removeItem(STORAGE_KEY);
        } finally {
            setIsLoading(false);
            setIsLogin(false);
            localStorage.removeItem(STORAGE_KEY);
        }
    }, []);

    const login = async (email: string, password: string): Promise<string | null> => {
        try {
            // // Replace with your real API call — this should return a token
            // const res = await fetch('/api/login', {
            //     method: 'POST',
            //     headers: { 'Content-Type': 'application/json' },
            //     body: JSON.stringify({ email, password }),
            // });

            // if (!res.ok) {
            //     return 'Invalid email or password';
            // }

            // const data = await res.json();
            localStorage.setItem(STORAGE_KEY, "yes");
            setIsLogin(true);
            return null; // null = success, no error
        } catch {
            return 'Something went wrong. Please try again.';
        }
    };

    const logout = async (): Promise<string | null> => {
        setIsLogin(false);
        localStorage.removeItem(STORAGE_KEY);
        return null;
    };

    return (
        <AuthContext.Provider value={{ isLogin, isLoading, login, logout }}>
            {children}
        </AuthContext.Provider>
    );
}

export function useAuth() {
    const context = useContext(AuthContext);
    if (context === undefined) {
        throw new Error('useAuth must be used within an AuthProvider');
    }
    return context;
}