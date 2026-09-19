import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { navigationItems } from "./navigation";
import { App } from "./App";

describe("App navigation", () => {
  it("renders every navigation item with a consistent vector icon", () => {
    const markup = renderToStaticMarkup(<App />);
    const icons = markup.match(/<svg[^>]*data-nav-icon="true"[^>]*>/g) ?? [];

    expect(icons).toHaveLength(navigationItems.length);
    for (const icon of icons) {
      expect(icon).toContain('width="18"');
      expect(icon).toContain('height="18"');
      expect(icon).toContain('aria-hidden="true"');
    }
    for (const item of navigationItems) {
      expect(markup).toContain(`aria-label="${item.label}"`);
      expect(markup).toContain(`title="${item.label}"`);
    }
  });
});
