import { HttpMethod } from './http';
import { ApiResponse, JsonCallResult } from './result';

const BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL ?? '';
const TOKEN_KEY = 'token';


/**
 * Joins a base URL and an endpoint path, ensuring exactly one slash between them
 * regardless of whether either side already has one.
 *
 * buildUrl('http://api.com', '/users')   -> 'http://api.com/users'
 * buildUrl('http://api.com/', 'users')   -> 'http://api.com/users'
 * buildUrl('http://api.com/', '/users')  -> 'http://api.com/users'
 * buildUrl('http://api.com', 'users')    -> 'http://api.com/users'
 */
export function buildUrl(base: string, endpoint: string): string {
    const trimmedBase = base.replace(/\/+$/, ''); // strip trailing slash(es)
    const trimmedEndpoint = endpoint.replace(/^\/+/, ''); // strip leading slash(es)
    return `${trimmedBase}/${trimmedEndpoint}`;
}

export function getToken(): string | null {
    if (typeof window === 'undefined') return null; // SSR guard
    try {
        return localStorage.getItem(TOKEN_KEY);
    } catch {
        return null;
    }
}

export async function jsonCall<TInput = null, TOutput = unknown>(
    endpoint: string,
    method: HttpMethod,
    input: TInput | null = null
): Promise<JsonCallResult<TOutput>> {
    const token = getToken();
    const url = buildUrl(BASE_URL, endpoint);
    const headers: HeadersInit = {
        'Content-Type': 'application/json',
        Accept: 'application/json',
    };

    if (token) {
        headers['Authorization'] = `Bearer ${token}`;
    }

    let response: Response;

    try {
        response = await fetch(url, {
            method,
            headers,
            body: input !== null && input !== undefined ? JSON.stringify(input) : undefined,
        });
    } catch (err) {
        // Network-level failure (offline, DNS, CORS, etc.) — no HTTP response at all
        return {
            res: null,
            error: {
                message: err instanceof Error ? err.message : 'Network request failed',
                status: 0,
                code: -1,
            },
        };
    }

    let body: ApiResponse<TOutput> | null = null;

    try {
        body = await response.json();
    } catch {
        // Response wasn't valid JSON at all
        return {
            res: null,
            error: {
                message: `Invalid JSON response (HTTP ${response.status})`,
                status: response.status,
                code: -1,
            },
        };
    }

    if (!body) {
        return {
            res: null,
            error: {
                message: `Empty response (HTTP ${response.status})`,
                status: response.status,
                code: -1,
            },
        };
    }

    // Server-level error: ApiResponse.error is populated
    if (body.error && body.error.length > 0) {
        return {
            res: null,
            error: {
                message: body.error.join(', '),
                status: body.status,
                code: -1,
            },
        };
    }

    // HTTP-level failure with no structured error body (fallback safety net)
    if (!response.ok) {
        return {
            res: null,
            error: {
                message: `Request failed (HTTP ${response.status})`,
                status: response.status,
                code: -1,
            },
        };
    }

    return { res: body.result, error: null };
}