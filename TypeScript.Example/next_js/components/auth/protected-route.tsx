'use client';

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/context/auth-context';

export function ProtectedRoute({ children }: { children: React.ReactNode }) {
    const { isLogin, isLoading } = useAuth();
    const router = useRouter();

    useEffect(() => {
        if (!isLoading && !isLogin) {
            router.replace('/login');
        }
    }, [isLoading, isLogin, router]);

    if (isLoading) return null;
    if (!isLogin) return null;

    return <>{children}</>;
}