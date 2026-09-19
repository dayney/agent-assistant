import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { StatusBadge } from "./StatusBadge";

describe("StatusBadge", () => {
  it("labels an unverified capability as unknown", () => {
    const markup = renderToStaticMarkup(<StatusBadge value="unknown" />);

    expect(markup).toContain("待核实");
    expect(markup).toContain("status-unknown");
  });
});
