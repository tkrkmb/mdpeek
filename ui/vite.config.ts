import { defineConfig } from "vitest/config";

export default defineConfig({
  // CDNからは何も読み込まないので、すべてをバンドルして相対パスで参照する
  base: "./",
  build: {
    target: "es2022",
    emptyOutDir: true,
  },
  clearScreen: false,
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
  },
});
