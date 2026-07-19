export interface AppError {
    message: string;
    status: number;
    code: number;
}

export interface ApiResponse<TOutput> {
    result: TOutput | null;
    error: string[] | null;
    status: number;
}

export interface JsonCallResult<TOutput> {
    res: TOutput | null;
    error: AppError | null;
}
