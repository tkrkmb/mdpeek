import { defineConfig } from "vite";

export default defineConfig({
  // CDNからは何も読み込まないので、すべてをバンドルして相対パスで参照する
  base: "./",
  build: {
    target: "es2022",
    emptyOutDir: true,
  },
  clearScreen: false,
});
