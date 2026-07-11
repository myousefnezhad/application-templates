import type { NextConfig } from "next";

const nextConfig = {
  output: 'standalone',
  turbopack: {
    root: __dirname,
  },
  reactCompiler: true,
  devIndicators: false,
};

export default nextConfig;
