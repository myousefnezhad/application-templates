'use client';

import { useEffect } from 'react';
import { useAuth } from '@/context/auth-context';
import { useRouter } from 'next/navigation';

export default function LoginPage() {
    const { isLogin, isLoading, login } = useAuth();
    const router = useRouter();

    // If already logged in, skip the login page entirely
    useEffect(() => {
        if (!isLoading && isLogin) {
            router.replace('/');
        }
    }, [isLoading, isLogin, router]);

    const handleLogin = async () => {
        const error = await login('username', 'password'); // Replace with actual credentials
        if (!error) {
            router.replace('/');
        }
    };

    // Avoid flashing the login form while we check auth state or redirect
    if (isLoading || isLogin) return null;

    return (
        <div className="flex flex-col items-center gap-4">
            <h1 className="text-2xl font-bold">Login</h1>
            <button
                type="button"
                onClick={handleLogin}
                className="border border-border rounded px-4 py-2"
            >
                Log in
            </button>
        </div>
    );
}