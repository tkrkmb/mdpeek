import { describe, expect, test } from "vitest";

import { abbreviateHome } from "./pathbar";

describe("abbreviateHome", () => {
  test("replaces the home directory with ~", () => {
    expect(abbreviateHome("/Users/me/docs/a.md", "/Users/me")).toBe("~/docs/a.md");
  });

  test("accepts a home directory with a trailing slash", () => {
    expect(abbreviateHome("/Users/me/a.md", "/Users/me/")).toBe("~/a.md");
  });

  test("leaves paths outside home alone", () => {
    expect(abbreviateHome("/tmp/a.md", "/Users/me")).toBe("/tmp/a.md");
  });

  test("does not treat a sibling with the same prefix as home", () => {
    expect(abbreviateHome("/Users/meg/a.md", "/Users/me")).toBe("/Users/meg/a.md");
  });

  test("keeps the path when home is unknown", () => {
    expect(abbreviateHome("/Users/me/a.md", null)).toBe("/Users/me/a.md");
  });
});
