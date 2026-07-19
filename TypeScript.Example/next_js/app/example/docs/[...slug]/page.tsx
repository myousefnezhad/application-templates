export default async function DocsPage({
    params,
}: {
    params: Promise<{ slug: string[] }>;
}) {
    const { slug } = await params;

    return (
        <main style={{ padding: "2rem" }}>
            <h1>Documentation</h1>

            <p>
                <strong>Slug:</strong> {slug.join(" / ")}
            </p>

            <p>
                <strong>Path:</strong> /docs/{slug.join("/")}
            </p>

            <pre>{JSON.stringify(slug, null, 2)}</pre>
        </main>
    );
}