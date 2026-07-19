'use client';

import React, { useMemo, useState } from 'react';
import {
    ColumnDef,
    flexRender,
    getCoreRowModel,
    getFilteredRowModel,
    getPaginationRowModel,
    getSortedRowModel,
    SortingState,
    useReactTable,
} from '@tanstack/react-table';

type Person = {
    id: number;
    name: string;
};

export default function Home() {
    const [data, setData] = useState<Person[]>([
        { id: 1, name: 'Tony' },
        { id: 2, name: 'Sarah' },
        { id: 3, name: 'Jake' },
        { id: 4, name: 'Eva' },
        { id: 5, name: 'Barry' },
        { id: 6, name: 'Russ' },
    ]);

    const [id, setId] = useState('');
    const [name, setName] = useState('');

    const [sorting, setSorting] = useState<SortingState>([]);
    const [globalFilter, setGlobalFilter] = useState('');

    const columns = useMemo<ColumnDef<Person>[]>(
        () => [
            {
                accessorKey: 'id',
                header: 'ID',
            },
            {
                accessorKey: 'name',
                header: 'Name',
            },
        ],
        []
    );

    const table = useReactTable({
        data,
        columns,
        state: {
            sorting,
            globalFilter,
        },
        onSortingChange: setSorting,
        onGlobalFilterChange: setGlobalFilter,
        getCoreRowModel: getCoreRowModel(),
        getFilteredRowModel: getFilteredRowModel(),
        getSortedRowModel: getSortedRowModel(),
        getPaginationRowModel: getPaginationRowModel(),
    });

    function addPerson() {
        if (!id || !name) return;

        setData((old) => [
            ...old,
            {
                id: Number(id),
                name,
            },
        ]);

        setId('');
        setName('');
    }

    return (
        <div
            style={{
                padding: 30,
                fontFamily: 'Arial',
                maxWidth: 900,
                margin: 'auto',
            }}
        >
            <h2>TanStack Table Demo</h2>

            <div
                style={{
                    display: 'flex',
                    gap: 10,
                    marginBottom: 20,
                }}
            >
                <input
                    placeholder="ID"
                    value={id}
                    onChange={(e) => setId(e.target.value)}
                />

                <input
                    placeholder="Name"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                />

                <button onClick={addPerson}>Add</button>

                <div style={{ flex: 1 }} />

                
            </div>

            <div>
                <input
                    placeholder="Search..."
                    value={globalFilter}
                    onChange={(e) => setGlobalFilter(e.target.value)}
                />
            </div>

            <table
                style={{
                    width: '100%',
                    borderCollapse: 'collapse',
                }}
            >
                <thead>
                    {table.getHeaderGroups().map((group) => (
                        <tr key={group.id}>
                            {group.headers.map((header) => (
                                <th
                                    key={header.id}
                                    onClick={header.column.getToggleSortingHandler()}
                                    style={{
                                        cursor: 'pointer',
                                        border: '1px solid #ccc',
                                        padding: 10,
                                        background: '#0b0b0b',
                                    }}
                                >
                                    {flexRender(
                                        header.column.columnDef.header,
                                        header.getContext()
                                    )}

                                    {{
                                        asc: ' 🔼',
                                        desc: ' 🔽',
                                    }[
                                        header.column.getIsSorted() as
                                        | 'asc'
                                        | 'desc'
                                    ] ?? ''}
                                </th>
                            ))}
                        </tr>
                    ))}
                </thead>

                <tbody>
                    {table.getRowModel().rows.map((row) => (
                        <tr key={row.id}>
                            {row.getVisibleCells().map((cell) => (
                                <td
                                    key={cell.id}
                                    style={{
                                        border: '1px solid #ddd',
                                        padding: 10,
                                    }}
                                >
                                    {flexRender(
                                        cell.column.columnDef.cell,
                                        cell.getContext()
                                    )}
                                </td>
                            ))}
                        </tr>
                    ))}
                </tbody>
            </table>

            <div
                style={{
                    marginTop: 20,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 10,
                }}
            >
                <button
                    onClick={() => table.firstPage()}
                    disabled={!table.getCanPreviousPage()}
                >
                    {'<<'}
                </button>

                <button
                    onClick={() => table.previousPage()}
                    disabled={!table.getCanPreviousPage()}
                >
                    Previous
                </button>

                <button
                    onClick={() => table.nextPage()}
                    disabled={!table.getCanNextPage()}
                >
                    Next
                </button>

                <button
                    onClick={() => table.lastPage()}
                    disabled={!table.getCanNextPage()}
                >
                    {'>>'}
                </button>

                <span>
                    Page {table.getState().pagination.pageIndex + 1} of{' '}
                    {table.getPageCount()}
                </span>
            </div>
        </div>
    );
}