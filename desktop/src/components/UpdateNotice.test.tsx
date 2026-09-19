import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { UpdateNotice } from "./UpdateNotice";
import type { UpdateState } from "../update/model";

const availableState: UpdateState = {
  status: "available",
  currentVersion: "0.0.20",
  candidate: {
    version: "0.0.21",
    notes: "修复更新流程",
    publishedAt: null,
  },
  progress: null,
  error: null,
};

describe("UpdateNotice", () => {
  it("offers immediate installation and a later choice", () => {
    const markup = renderToStaticMarkup(
      <UpdateNotice
        state={availableState}
        hasUnsavedChanges={false}
        autoOpen={false}
        onCheck={async () => undefined}
        onInstall={async () => undefined}
        onDismiss={() => undefined}
      />,
    );

    expect(markup).toContain("立即更新");
    expect(markup).toContain("稍后");
  });
});
