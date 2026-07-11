import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: 'standalone',
  turbopack: {
    root: __dirname,
  },
  reactCompiler: true,
  devIndicators: false,
};

export default nextConfig;
