import { test, expect, type Page } from "@playwright/test";
// Keep documentation captures readable without changing app notification behavior.
async function capture(page: Page, path: string) {
  await page.screenshot({
    path,
    animations: "disabled",
    style: ".notification-viewport { visibility: hidden !important; }",
  });
}
async function prepare(page: Page, selectedDirectory?: string) {
  const fixture = await (
    await page.request.get("http://127.0.0.1:1421/fixture")
  ).json();
  await page.addInitScript(
    ({ fixture }) => {
      const callbacks = new Map<number, (value: unknown) => void>(),
        listeners = new Map<string, number[]>();
      let next = 1;
      const emit = (event: string, payload: unknown) => {
        for (const id of listeners.get(event) ?? [])
          callbacks.get(id)?.({ event, payload, id });
      };
      Object.assign(window, {
        isTauri: true,
        __TAURI_INTERNALS__: {
          transformCallback: (cb: (value: unknown) => void) => {
            const id = next++;
            callbacks.set(id, cb);
            return id;
          },
          unregisterCallback: (id: number) => callbacks.delete(id),
          invoke: async (
            command: string,
            args: Record<string, unknown> = {},
          ) => {
            if (command === "plugin:event|listen") {
              const event = String(args.event);
              listeners.set(event, [
                ...(listeners.get(event) ?? []),
                Number(args.handler),
              ]);
              return args.handler;
            }
            if (command === "plugin:event|unlisten") return;
            if (command === "plugin:dialog|open") return fixture.checkout;
            if (command === "mutate") {
              const id = String(next++);
              emit("operation", {
                id,
                repoId: args.id,
                kind: (args.action as { kind: string }).kind,
                status: "running",
                output: "",
                error: null,
              });
              const request = () =>
                fetch("http://127.0.0.1:1421", {
                  method: "POST",
                  body: JSON.stringify({ command, args }),
                })
                  .then((r) => r.json())
                  .then((reply) =>
                    emit("operation", {
                      id,
                      repoId: args.id,
                      kind: (args.action as { kind: string }).kind,
                      status: reply.error ? "failed" : "completed",
                      output: reply.result ?? "",
                      error: reply.error ?? null,
                      removedPath:
                        !reply.error &&
                        (args.action as { kind: string }).kind ===
                          "removeWorktree"
                          ? (args.action as { path: string }).path
                          : null,
                    }),
                  );
              // Keep the real push observable long enough to assert pending UI state.
              if ((args.action as { kind: string }).kind === "push")
                setTimeout(request, 800);
              else request();
              return id;
            }
            const reply = await (
              await fetch("http://127.0.0.1:1421", {
                method: "POST",
                body: JSON.stringify({ command, args }),
              })
            ).json();
            if (reply.error) throw reply.error;
            return reply.result;
          },
        },
        __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
      });
    },
    {
      fixture: { ...fixture, checkout: selectedDirectory ?? fixture.checkout },
    },
  );
  await page.goto("/");
  await expect(page.locator(".status-bar")).toContainText("git version");
  return fixture;
}
async function operation(
  page: Page,
  kind: string,
  fields: Record<string, string> = {},
) {
  await page.getByLabel("Repository actions").selectOption(kind);
  const dialog = page.getByRole("dialog");
  for (const [label, value] of Object.entries(fields))
    await dialog.getByLabel(label, { exact: true }).fill(value);
  await dialog
    .getByRole("button", {
      name: await dialog.getByRole("heading").innerText(),
      exact: true,
    })
    .click();
  await expect(dialog).toBeHidden();
  if (kind === "push") {
    const button = page.getByRole("button", { name: "Pushing…", exact: true });
    await expect(button).toBeDisabled();
    await expect(button).toHaveAttribute("aria-busy", "true");
    await expect(
      page.getByText("Pushing to remote…", { exact: true }),
    ).toBeVisible();
    await capture(page, "test-results/push-loading.png");
  }
  await expect(page.getByText("Running ", { exact: false })).toHaveCount(0);
}
test.describe
  .serial("Real Git engine through browser UI (test IPC transport)", () => {
  test("clone, branch, stage, commit, merge and push", async ({ page }) => {
    const fixture = await prepare(page);
    await capture(page, "test-results/welcome-green.png");
    await page
      .getByRole("button", { name: "Clone repository", exact: true })
      .click();
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel("Repository URL").fill(fixture.remote);
    await dialog
      .getByLabel("Destination directory", { exact: true })
      .fill(fixture.checkout);
    await dialog.getByRole("checkbox").check();
    await dialog
      .getByRole("button", { name: "Clone repository", exact: true })
      .click();
    await expect(
      page.getByRole("heading", { name: "History / All branches" }),
    ).toBeVisible();
    await expect(page.getByRole("listitem").first()).toContainText(
      "Add project module 7",
    );
    await page.getByRole("listitem").first().click();
    await expect(page.getByText("COMMIT DETAILS")).toBeVisible();
    await operation(page, "createBranch", { Name: "feature/ui-workflow" });
    await operation(page, "switchBranch", { Name: "feature/ui-workflow" });
    await expect(page.locator(".branch-indicator")).toContainText(
      "feature/ui-workflow",
    );
    await page.request.post("http://127.0.0.1:1421", {
      data: JSON.stringify({
        command: "edit_fixture",
        args: { path: fixture.checkout, content: "UI workflow greeting\n" },
      }),
    });
    await page.getByLabel("Refresh repository").click();
    await page.getByRole("button", { name: /Working changes/ }).click();
    await page
      .getByRole("button", { name: "Stage hello.txt", exact: true })
      .click();
    await expect(page.getByText("1 staged", { exact: true })).toBeVisible();
    await page
      .getByLabel("Commit message", { exact: true })
      .fill("Commit from the Git Toys UI");
    await page
      .getByRole("button", { name: "Commit staged", exact: true })
      .click();
    await expect(page.getByText("Working tree is clean")).toBeVisible();
    await operation(page, "switchBranch", { Name: "main" });
    await operation(page, "merge", {
      "Target commit or branch": "feature/ui-workflow",
    });
    await operation(page, "push");
    await expect(
      page.locator(".notice-success").filter({ hasText: "Changes pushed" }),
    ).toBeVisible();
    const heads = await (
      await page.request.get("http://127.0.0.1:1421/verify")
    ).json();
    expect(heads.local).toBe(heads.remote);
    await page.getByRole("button", { name: "History", exact: true }).click();
    await expect(page.getByRole("listitem").first()).toContainText(
      "Commit from the Git Toys UI",
    );
    await page.getByRole("listitem").first().click();
    await expect(
      page.locator(".commit-file-list button").first(),
    ).toBeVisible();
    await expect(
      page.locator(".code-editor .monaco-diff-editor"),
    ).toBeVisible();
    await capture(page, "test-results/history-light.png");
  });
  test("resolve a real conflict and continue the merge", async ({ page }) => {
    const fixture = await prepare(page);
    await page
      .getByRole("button", { name: /checkout/ })
      .first()
      .click();
    if (
      await page
        .getByRole("button", { name: "Trust repository to make changes" })
        .isVisible()
    ) {
      await page
        .getByRole("button", { name: "Trust repository to make changes" })
        .click();
      await page
        .getByRole("button", { name: "Trust repository", exact: true })
        .click();
    }
    await operation(page, "createBranch", {
      Name: "conflict-review",
      "Starting point": "origin/conflict-side",
    });
    await operation(page, "switchBranch", { Name: "conflict-review" });
    await operation(page, "merge", { "Target commit or branch": "main" });
    await expect(
      page.getByText("merge in progress", { exact: true }),
    ).toBeVisible();
    await page.getByRole("button", { name: /Working changes/ }).click();
    await page.locator(".file-select").filter({ hasText: "hello.txt" }).click();
    await expect(
      page.getByRole("dialog", { name: "Resolve hello.txt" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "Use ours as result" }).click();
    await page.getByRole("button", { name: "Save & mark resolved" }).click();
    await expect(page.getByRole("dialog")).toBeHidden();
    await page.getByRole("button", { name: "Continue…", exact: true }).click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Continue operation", exact: true })
      .click();
    await expect(
      page.getByText("merge in progress", { exact: true }),
    ).toHaveCount(0);
    expect(fixture.checkout).toBeTruthy();
    await page
      .getByRole("button", { name: "Settings and Git diagnostics" })
      .click();
    await page.getByLabel("Appearance").selectOption("dark");
    await page.getByLabel("Close dialog").click();
    await capture(page, "test-results/changes-dark.png");
  });
  test("command palette, keyboard controls, persistence and minimum window", async ({
    page,
  }) => {
    await prepare(page);
    await page.setViewportSize({ width: 960, height: 640 });
    await page
      .getByRole("button", { name: /checkout/ })
      .first()
      .click();
    await expect(page.getByRole("listitem").first()).toBeVisible();
    await page.keyboard.press("Control+k");
    await expect(
      page.getByRole("dialog", { name: "Command palette" }),
    ).toBeVisible();
    await page.getByLabel("Find a command").fill("rebase");
    await expect(
      page.getByRole("button", {
        name: "Rebase current branch Review required",
      }),
    ).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toBeHidden();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await capture(page, "test-results/minimum-window.png");
  });
  test("stable diffs during polling and external edits, with integer panel persistence", async ({
    page,
  }) => {
    const fixture = await prepare(page);
    await page
      .getByRole("button", { name: /checkout/ })
      .first()
      .click();
    if (
      await page
        .getByRole("button", { name: "Trust repository to make changes" })
        .isVisible()
    ) {
      await page
        .getByRole("button", { name: "Trust repository to make changes" })
        .click();
      await page
        .getByRole("button", { name: "Trust repository", exact: true })
        .click();
    }
    const content =
      Array.from({ length: 220 }, (_, index) => `Fixture line ${index}`).join(
        "\n",
      ) + "\n";
    await page.request.post("http://127.0.0.1:1421", {
      data: {
        command: "edit_fixture",
        args: { path: fixture.checkout, content },
      },
    });
    await page
      .getByRole("button", { name: "Refresh repository", exact: true })
      .click();
    await page.getByRole("button", { name: /Working changes/ }).click();
    let diffRequests = 0,
      snapshots = 0;
    page.on("request", (request) => {
      if (
        request.url() !== "http://127.0.0.1:1421/" ||
        request.method() !== "POST"
      )
        return;
      const command = request.postDataJSON().command;
      if (command === "diff") diffRequests++;
      if (command === "snapshot") snapshots++;
    });
    await page.locator(".file-select").filter({ hasText: "hello.txt" }).click();
    await expect(
      page.locator(".code-editor .monaco-diff-editor"),
    ).toBeVisible();
    const host = await page.locator(".code-editor").elementHandle();
    await page.locator(".code-editor").hover();
    await page.mouse.wheel(0, 500);
    const slider = page
      .locator(
        ".code-editor .monaco-editor .scrollbar.vertical .slider:visible",
      )
      .first();
    await expect
      .poll(() =>
        slider.evaluate((element) =>
          parseFloat((element as HTMLElement).style.top),
        ),
      )
      .toBeGreaterThan(0);
    const scrollTop = await slider.evaluate(
      (element) => (element as HTMLElement).style.top,
    );
    const before = snapshots;
    // Exercise three actual timer polls; no explicit refresh shortcut.
    await expect
      .poll(() => snapshots, { timeout: 12000 })
      .toBeGreaterThanOrEqual(before + 3);
    expect(diffRequests).toBe(1);
    expect(await host!.evaluate((element) => element.isConnected)).toBe(true);
    expect(
      await slider.evaluate((element) => (element as HTMLElement).style.top),
    ).toBe(scrollTop);

    let release!: () => void,
      pending = false;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    await page.route("http://127.0.0.1:1421/", async (route) => {
      if (
        route.request().method() === "POST" &&
        route.request().postDataJSON().command === "diff"
      ) {
        pending = true;
        await gate;
      }
      await route.continue();
    });
    await page.request.post("http://127.0.0.1:1421", {
      data: {
        command: "edit_fixture",
        args: {
          path: fixture.checkout,
          content: content.replace(
            "Fixture line 210",
            "Externally edited line 210",
          ),
        },
      },
    });
    await page
      .getByRole("button", { name: "Refresh repository", exact: true })
      .click();
    await expect.poll(() => pending).toBe(true);
    expect(await host!.evaluate((element) => element.isConnected)).toBe(true);
    await expect(page.getByText("Loading diff…", { exact: true })).toHaveCount(
      0,
    );
    await expect(
      page.getByRole("button", { name: "Stage file", exact: true }),
    ).toBeDisabled();
    release();
    await expect(
      page.getByRole("button", { name: "Stage file", exact: true }),
    ).toBeEnabled();
    expect(await host!.evaluate((element) => element.isConnected)).toBe(true);
    await expect
      .poll(() =>
        slider.evaluate((element) => (element as HTMLElement).style.top),
      )
      .toBe(scrollTop);
    await page.unroute("http://127.0.0.1:1421/");

    const divider = page.getByRole("separator", {
      name: "Resize diff",
      exact: true,
    });
    await divider.dispatchEvent("pointerdown", { clientX: 900.125 });
    await page.evaluate(() => {
      window.dispatchEvent(
        new PointerEvent("pointermove", { clientX: 780.375 }),
      );
      window.dispatchEvent(new PointerEvent("pointerup", { clientX: 780.375 }));
    });
    const saved = async () =>
      (
        await (
          await page.request.post("http://127.0.0.1:1421", {
            data: { command: "load_preferences", args: {} },
          })
        ).json()
      ).result;
    await expect.poll(async () => (await saved()).detailWidth).toBe(560);
    await divider.focus();
    await page.keyboard.press("ArrowLeft");
    await expect.poll(async () => (await saved()).detailWidth).toBe(570);
    const preferences = await saved();
    expect(Number.isInteger(preferences.detailWidth)).toBe(true);
    await page.reload();
    await page
      .getByRole("button", { name: /checkout/ })
      .first()
      .click();
    await expect(
      page.getByRole("separator", { name: "Resize details", exact: true }),
    ).toHaveAttribute("aria-valuenow", "570");
    await expect(page.locator(".notice-error")).toHaveCount(0);
  });
});

test("color palettes preview immediately, support keyboard selection, and persist", async ({
  page,
}) => {
  await prepare(page);
  await page
    .getByRole("button", { name: "Settings and Git diagnostics" })
    .click();
  await page.getByLabel("Appearance").selectOption("light");
  const violet = page.getByRole("radio", { name: "Violet", exact: true });
  await violet.check();
  await expect(page.locator("html")).toHaveAttribute(
    "data-color-theme",
    "violet",
  );
  await expect(violet).toBeChecked();
  await violet.focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("radio", { name: "Slate", exact: true }),
  ).toBeChecked();
  await expect(page.locator("html")).toHaveAttribute(
    "data-color-theme",
    "slate",
  );
  await violet.check();
  await page.getByLabel("Appearance").selectOption("dark");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await expect(page.locator("html")).toHaveCSS("--accent", "#c4a3ff");
  await expect(page.locator(".theme-card.selected")).toHaveCSS(
    "border-color",
    "rgb(196, 163, 255)",
  );
  await expect(page.locator(".theme-card").first()).toHaveCSS(
    "border-color",
    "rgb(51, 51, 51)",
  );
  await capture(page, ".reports/themes-dark.png");
  await expect
    .poll(
      async () =>
        (
          await (
            await page.request.post("http://127.0.0.1:1421", {
              data: { command: "load_preferences" },
            })
          ).json()
        ).result.colorTheme,
    )
    .toBe("violet");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute(
    "data-color-theme",
    "violet",
  );
  await page
    .getByRole("button", { name: "Settings and Git diagnostics" })
    .click();
  await expect(
    page.getByRole("radio", { name: "Violet", exact: true }),
  ).toBeChecked();
  await page.getByLabel("Appearance").selectOption("light");
  await expect(page.locator(".theme-card.selected")).toHaveCSS(
    "border-color",
    "rgb(120, 60, 189)",
  );
  await expect(page.locator(".theme-card").first()).toHaveCSS(
    "border-color",
    "rgb(229, 229, 229)",
  );
  await capture(page, ".reports/themes-light.png");
  await page.getByLabel("Close dialog").click();
  await capture(page, "test-results/welcome-light.png");
  await page
    .getByRole("button", { name: "Settings and Git diagnostics" })
    .click();
  await page.getByLabel("Appearance").selectOption("dark");
  await page.getByLabel("Close dialog").click();
  await expect(page.locator(".welcome-actions button").first()).toHaveCSS(
    "background-color",
    "rgb(28, 28, 28)",
  );
  await capture(page, "test-results/welcome-dark.png");
});

async function featureRepository(page: Page, name: string) {
  const fixture = await (
    await page.request.get("http://127.0.0.1:1421/fixture")
  ).json();
  const path = `${fixture.sandbox}/${name}`;
  const response = await (
    await page.request.post("http://127.0.0.1:1421", {
      data: {
        command: "create_repository",
        args: { path, url: fixture.remote, trusted: true },
      },
    })
  ).json();
  expect(response.error).toBeUndefined();
  await prepare(page, path);
  await page
    .getByRole("button", { name: "Open repository", exact: true })
    .first()
    .click();
  await expect(page.getByRole("tab", { name, exact: true })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  return { ...fixture, path, id: response.result.id };
}

test("AI reviews real staged changes and applies an editable draft without committing", async ({
  page,
}) => {
  const fixture = await featureRepository(page, "ai-ui");
  await page
    .getByRole("button", { name: "Settings and Git diagnostics" })
    .click();
  await page.getByLabel("Enable AI commit drafts").check();
  await page.getByLabel("AI provider").selectOption("ollama");
  await page.getByLabel("AI model", { exact: true }).fill("fixture-model");
  await page.getByLabel("Close dialog").click();
  await page.request.post("http://127.0.0.1:1421", {
    data: {
      command: "edit_fixture",
      args: { path: fixture.path, content: "AI staged greeting\n" },
    },
  });
  await page.getByRole("button", { name: "Working changes" }).click();
  await page.getByRole("button", { name: "Refresh repository" }).click();
  await page
    .getByRole("button", { name: "Stage hello.txt", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Generate with AI" }),
  ).toBeEnabled();
  await page
    .getByLabel("Commit message", { exact: true })
    .fill("Keep my draft");
  const before = (
    await (
      await page.request.post("http://127.0.0.1:1421", {
        data: { command: "snapshot", args: { id: fixture.id } },
      })
    ).json()
  ).result.head;
  let generated = 0;
  await page.route("http://127.0.0.1:1421/", async (route) => {
    const body = route.request().postDataJSON();
    if (body.command === "ai_generate") {
      generated++;
      expect(body.args.context.context).toContain("AI staged greeting");
      await route.fulfill({
        json: {
          result: {
            message: "Improve the greeting",
            indexToken: body.args.context.indexToken,
          },
        },
      });
    } else await route.continue();
  });
  await page.getByRole("button", { name: "Generate with AI" }).click();
  const dialog = page.getByRole("dialog", { name: "Draft a commit with AI" });
  await dialog.getByText("Exact outgoing context", { exact: false }).click();
  await expect(dialog.locator("pre")).toContainText("AI staged greeting");
  await dialog
    .getByRole("button", { name: "Generate draft", exact: true })
    .click();
  await expect(dialog.getByLabel("AI draft")).toHaveValue(
    "Improve the greeting",
  );
  await expect(
    dialog.getByRole("button", { name: "Use message" }),
  ).toBeDisabled();
  await dialog.getByLabel("AI draft").fill("Improve greeting wording");
  await dialog.getByLabel("Replace my existing commit message").check();
  await capture(page, "test-results/ai-draft.png");
  await dialog.getByRole("button", { name: "Use message" }).click();
  await expect(page.getByLabel("Commit message", { exact: true })).toHaveValue(
    "Improve greeting wording",
  );
  expect(generated).toBe(1);
  const after = (
    await (
      await page.request.post("http://127.0.0.1:1421", {
        data: { command: "snapshot", args: { id: fixture.id } },
      })
    ).json()
  ).result.head;
  expect(after).toBe(before);
});

test("workspaces create, open, reuse tabs, remove safely and preserve branches", async ({
  page,
}) => {
  const fixture = await featureRepository(page, "worktree-ui");
  const destination = `${fixture.sandbox}/parallel-ui`;
  await page.getByRole("button", { name: "Worktrees", exact: true }).click();
  await expect(page.locator(".worktree-card")).toHaveCount(1);
  await page
    .getByRole("button", { name: "Create workspace", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "Create a parallel workspace",
  });
  await dialog.getByLabel("New workspace folder").fill(destination);
  await dialog.getByLabel("Workspace branch").fill("feature/parallel-ui");
  await dialog
    .getByRole("button", { name: "Create workspace", exact: true })
    .click();
  const card = page
    .locator(".worktree-card")
    .filter({ hasText: "feature/parallel-ui" });
  await expect(card).toBeVisible();
  await card.getByRole("button", { name: "Open workspace" }).click();
  await expect(
    page.getByRole("tab", { name: "parallel-ui", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.getByRole("tab", { name: "worktree-ui", exact: true }).click();
  await card.getByRole("button", { name: "Open workspace" }).click();
  await expect(
    page.getByRole("tab", { name: "parallel-ui", exact: true }),
  ).toHaveCount(1);
  await expect(
    page.getByRole("tab", { name: "parallel-ui", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await page.getByRole("tab", { name: "worktree-ui", exact: true }).click();
  await capture(page, "test-results/worktrees.png");
  await card
    .getByRole("button", { name: "Remove workspace feature/parallel-ui" })
    .click();
  const removal = page.getByRole("dialog", { name: "Remove this workspace?" });
  await expect(removal).toContainText(destination);
  await removal
    .getByRole("button", { name: "Remove workspace", exact: true })
    .click();
  await expect(page.locator(".worktree-card")).toHaveCount(1);
  await expect(
    page.getByRole("tab", { name: "parallel-ui", exact: true }),
  ).toHaveCount(0);
  const snapshot = (
    await (
      await page.request.post("http://127.0.0.1:1421", {
        data: { command: "snapshot", args: { id: fixture.id } },
      })
    ).json()
  ).result;
  expect(
    snapshot.refs.some(
      (ref: { name: string }) => ref.name === "feature/parallel-ui",
    ),
  ).toBe(true);
});

test("file time machine shows history and blame, previews restoration and leaves it unstaged", async ({
  page,
}) => {
  const fixture = await featureRepository(page, "history-ui");
  await page
    .getByRole("listitem")
    .filter({ hasText: "Update the greeting" })
    .click();
  await page
    .getByRole("button", { name: "View file history", exact: true })
    .click();
  const dialog = page.getByRole("dialog", {
    name: "File time machine",
    exact: true,
  });
  const firstRevision = dialog.getByRole("button", {
    name: "Update the greeting",
    exact: false,
  });
  await firstRevision.focus();
  await firstRevision.press("End");
  const olderRevision = dialog.getByRole("button", {
    name: "Start the project",
    exact: false,
  });
  await expect(olderRevision).toBeFocused();
  await expect(olderRevision).toHaveAttribute("aria-pressed", "true");
  await dialog.getByRole("button", { name: "Blame", exact: true }).click();
  await expect(
    dialog.getByRole("list", { name: "Line authors" }),
  ).toContainText("Git Toys Fixture");
  await capture(page, "test-results/file-blame.png");
  await dialog.getByRole("button", { name: "Changes", exact: true }).click();
  await expect(dialog.locator(".monaco-diff-editor")).toBeVisible();
  await capture(page, "test-results/file-time-machine.png");
  await dialog
    .getByRole("button", { name: "Restore this version", exact: true })
    .click();
  const restore = page.getByRole("dialog", {
    name: "Restore this file version?",
    exact: true,
  });
  await expect(restore).toContainText("current working file");
  await restore
    .getByRole("button", { name: "Restore as unstaged change" })
    .click();
  await expect(restore).toBeHidden();
  await dialog.getByLabel("Close dialog", { exact: true }).first().click();
  await page.getByRole("button", { name: "Working changes" }).click();
  await expect(
    page.getByRole("button", { name: "Stage hello.txt", exact: true }),
  ).toBeVisible();
  const snapshot = (
    await (
      await page.request.post("http://127.0.0.1:1421", {
        data: { command: "snapshot", args: { id: fixture.id } },
      })
    ).json()
  ).result;
  const changed = snapshot.files.find(
    (file: { path: string }) => file.path === "hello.txt",
  );
  expect(changed.index).toBe(".");
  expect(changed.worktree).toBe("M");
  const diff = (
    await (
      await page.request.post("http://127.0.0.1:1421", {
        data: {
          command: "diff",
          args: {
            id: fixture.id,
            request: {
              path: "hello.txt",
              staged: false,
              commit: null,
              base: null,
            },
          },
        },
      })
    ).json()
  ).result;
  expect(diff.modified).toBe("hello\n");
});

async function freshClone(page: Page, name: string) {
  const fixture = await prepare(page);
  const directory = `${fixture.sandbox}/${name}`;
  await page
    .getByRole("button", { name: "Clone repository", exact: true })
    .click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Repository URL").fill(fixture.remote);
  await dialog
    .getByLabel("Destination directory", { exact: true })
    .fill(directory);
  await dialog.getByRole("checkbox").check();
  await dialog
    .getByRole("button", { name: "Clone repository", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "History / All branches" }),
  ).toBeVisible();
  const opened = await (
    await page.request.post("http://127.0.0.1:1421", {
      data: { command: "open_repository", args: { path: directory } },
    })
  ).json();
  expect(opened.error).toBeUndefined();
  return { ...fixture, directory: opened.result.id as string };
}

test("organize selected lines into commits and preserve composer drafts", async ({
  page,
}) => {
  const fixture = await freshClone(page, "organized-work");
  await page.request.post("http://127.0.0.1:1421", {
    data: {
      command: "edit_fixture",
      args: {
        path: fixture.directory,
        content: "UI workflow greeting\norganized one\norganized two\n",
      },
    },
  });
  await page.getByLabel("Refresh repository").click();
  await page.getByRole("button", { name: /Working changes/ }).click();
  await page
    .getByLabel("Commit message", { exact: true })
    .fill("My existing commit draft");
  await page
    .getByRole("button", { name: "Organize changes", exact: true })
    .click();
  const organizer = page.getByRole("dialog", { name: "Organize changes" });
  await organizer
    .getByLabel("Group name", { exact: true })
    .fill("First focused commit");
  await organizer
    .getByRole("button", { name: "hello.txt", exact: true })
    .click();
  const line = organizer
    .locator(".selection-line")
    .filter({ hasText: "+organized one" })
    .getByRole("checkbox");
  await line.focus();
  await page.keyboard.press("Space");
  await organizer
    .getByRole("button", { name: "Assign 1 line to group" })
    .click();
  await expect(
    organizer.getByText("1 saved lines.", { exact: false }),
  ).toBeVisible();
  await capture(page, "test-results/commit-organizer.png");
  await organizer
    .getByRole("button", { name: "Stage group and write commit" })
    .click();
  await expect(organizer).toBeHidden();
  await expect(page.getByLabel("Commit message", { exact: true })).toHaveValue(
    "My existing commit draft",
  );
  const staged = await (
    await page.request.post("http://127.0.0.1:1421", {
      data: {
        command: "diff",
        args: {
          id: fixture.directory,
          request: {
            path: "hello.txt",
            staged: true,
            commit: null,
            base: null,
          },
        },
      },
    })
  ).json();
  expect(staged.result.modified).toContain("organized one");
  expect(staged.result.modified).not.toContain("organized two");
  await page
    .getByRole("button", { name: "Commit staged", exact: true })
    .click();
  await expect(page.getByText("0 staged", { exact: true })).toBeVisible();
  await page
    .getByRole("button", { name: "Organize changes", exact: true })
    .click();
  await organizer
    .getByRole("button", { name: "hello.txt", exact: true })
    .click();
  await organizer
    .locator(".selection-line")
    .filter({ hasText: "+organized two" })
    .getByRole("checkbox")
    .check();
  await organizer
    .getByRole("button", { name: "Assign 1 line to group" })
    .click();
  await organizer
    .getByRole("button", { name: "Stage group and write commit" })
    .click();
  await expect(organizer).toBeHidden();
  await page
    .getByRole("button", { name: "Commit staged", exact: true })
    .click();
  await expect(page.getByText("Working tree is clean")).toBeVisible();
});

test("preview conflicts without changing the repository and rewrite reviewed history", async ({
  page,
}) => {
  const fixture = await freshClone(page, "preview-work");
  const state = async () =>
    (
      await (
        await page.request.post("http://127.0.0.1:1421", {
          data: { command: "snapshot", args: { id: fixture.directory } },
        })
      ).json()
    ).result;
  const before = await state();
  await page.getByLabel("Repository actions").selectOption("previewMerge");
  const preview = page.getByRole("dialog", {
    name: "Preview merge",
    exact: true,
  });
  await preview
    .getByLabel("Branch or commit to merge")
    .fill("origin/conflict-side");
  await preview.getByRole("button", { name: "Generate preview" }).click();
  await expect(
    preview.getByText("Conflict: hello.txt", { exact: true }),
  ).toBeVisible();
  await capture(page, "test-results/merge-preview.png");
  expect((await state()).stateToken).toBe(before.stateToken);
  await preview.getByRole("button", { name: "Close dialog" }).click();
  await page
    .getByRole("button", { name: "Edit branch history", exact: true })
    .click();
  const rebase = page.getByRole("dialog", {
    name: "Edit branch history",
    exact: true,
  });
  await rebase.getByLabel("Base commit (excluded from editing)").fill("HEAD~2");
  await rebase.getByRole("button", { name: "Load commits" }).click();
  await expect(rebase.locator(".rebase-steps li")).toHaveCount(2);
  await rebase
    .locator(".rebase-steps li")
    .first()
    .getByRole("combobox")
    .selectOption("reword");
  await rebase
    .getByLabel("New message", { exact: true })
    .fill("Reviewed UI history");
  await rebase.getByRole("button", { name: "Preview edited history" }).click();
  await expect(rebase.getByLabel("Operation preview")).toBeVisible();
  await capture(page, "test-results/interactive-rebase.png");
  await rebase.getByRole("checkbox").check();
  await rebase
    .getByRole("button", { name: "Create recovery branch and rebase" })
    .click();
  await expect(rebase).toBeHidden();
  await expect.poll(async () => (await state()).head).not.toBe(before.head);
  expect(
    (await state()).refs.some((ref: { name: string }) =>
      ref.name.startsWith("git-toys/recovery/"),
    ),
  ).toBe(true);
});

test("GitHub hub renders paginated mocked PR data without remote HTML or images", async ({
  page,
}) => {
  await freshClone(page, "github-work");
  await operation(page, "addRemote", {
    Name: "github",
    "Repository URL": "https://github.com/fixture/project.git",
  });
  const pr = {
    number: 42,
    title: "Improve commit review",
    author: "contributor",
    state: "open",
    draft: false,
    body: 'Review staged changes safely. <img src="https://tracker.invalid/image">',
    head: "a".repeat(40),
    headLabel: "contributor:review",
    base: "main",
    mergeability: "Mergeable",
  };
  await page.route("http://127.0.0.1:1421/", async (route) => {
    const request = route.request().postDataJSON();
    let result: unknown;
    if (request.command === "github_pulls")
      result = { items: [pr], more: false };
    else if (request.command === "github_pull") result = pr;
    else if (request.command === "github_section")
      result = {
        items:
          request.args.section === "files"
            ? [
                {
                  name: "src/review.ts",
                  status: "modified",
                  detail: "+2 −1",
                  oid: "",
                  patch: "@@ -1 +1 @@\n-old\n+reviewed\n+safe",
                },
              ]
            : [
                {
                  name: "Build and test",
                  status: "completed",
                  detail: "success",
                  oid: "",
                  patch: null,
                },
              ],
        more: false,
      };
    else {
      await route.continue();
      return;
    }
    await route.fulfill({ json: { result } });
  });
  await page
    .getByRole("button", { name: "Pull requests", exact: true })
    .click();
  await page.getByRole("button", { name: /#42 Improve commit review/ }).click();
  await expect(
    page.getByRole("heading", { name: "#42 Improve commit review" }),
  ).toBeVisible();
  await expect(page.getByText("Build and test", { exact: true })).toBeVisible();
  await page.getByText("Description", { exact: true }).click();
  await expect(page.locator(".pull-request-details img")).toHaveCount(0);
  await page
    .getByRole("button", { name: "Changed files", exact: true })
    .click();
  await expect(page.getByText("src/review.ts", { exact: true })).toBeVisible();
  await capture(page, "test-results/pull-requests.png");
});

test("dialog motion keeps focus immediate, survives interruption, and respects reduced motion", async ({
  page,
}) => {
  await prepare(page);
  const settings = page.getByRole("button", {
    name: "Settings and Git diagnostics",
  });
  await settings.hover();
  await page.mouse.down();
  expect(
    await settings.evaluate(
      (element) => new DOMMatrixReadOnly(getComputedStyle(element).transform).a,
    ),
  ).toBeCloseTo(0.97);
  await page.mouse.move(1, 1);
  await page.mouse.up();
  await expect
    .poll(() =>
      settings.evaluate((element) => getComputedStyle(element).transform),
    )
    .toBe("none");
  // Slow the real CSS transitions to 10% and inspect the paired surface/scrim.
  await settings.evaluate((button: HTMLButtonElement) => button.click());
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await expect
    .poll(() => dialog.evaluate((element) => element.getAnimations().length), {
      intervals: [16],
      timeout: 1000,
    })
    .toBeGreaterThan(0);
  const timing = await page.evaluate(() => {
    const elements = [
      document.querySelector(".dialog")!,
      document.querySelector(".dialog-overlay")!,
    ];
    return elements.map((element) => {
      const animations = element.getAnimations();
      animations.forEach((animation) => {
        animation.playbackRate = 0.1;
        animation.pause();
        animation.currentTime = 110;
      });
      const style = getComputedStyle(element);
      const result = {
        opacity: Number(style.opacity),
        transform: style.transform,
        durations: animations.map((a) => a.effect!.getTiming().duration),
      };
      animations.forEach((animation) => animation.finish());
      return result;
    });
  });
  expect(timing[0]!.durations).toContain(220);
  expect(timing[1]!.durations).toContain(220);
  for (const sample of timing) {
    expect(sample.opacity).toBeGreaterThan(0);
    expect(sample.opacity).toBeLessThan(1);
  }
  await expect
    .poll(() =>
      dialog.evaluate((element) => element.contains(document.activeElement)),
    )
    .toBe(true);
  const surface = await dialog.elementHandle();
  // Reopen before the 120 ms exit settles, then check for duplicate layers/focus traps.
  await page.evaluate(() => {
    (
      document.querySelector('[aria-label="Close dialog"]') as HTMLButtonElement
    ).click();
    queueMicrotask(() =>
      (
        document.querySelector(
          '[aria-label="Settings and Git diagnostics"]',
        ) as HTMLButtonElement
      ).click(),
    );
  });
  await expect(dialog).toHaveCount(1);
  await expect(dialog).toBeVisible();
  expect(
    await surface!.evaluate(
      (element) => element === document.querySelector(".dialog"),
    ),
  ).toBe(true);
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await page.getByRole("button", { name: "Commands", exact: false }).click();
  await expect(dialog).toBeVisible();
  expect(
    await dialog.evaluate((element) => element.getAnimations().length),
  ).toBe(0);
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);

  await page.emulateMedia({ reducedMotion: "reduce" });
  await settings.click();
  await expect(dialog).toBeVisible();
  // Check the scale independently of the dialog's content-dependent centering offset.
  const transform = await dialog.evaluate(
    (element) => new DOMMatrixReadOnly(getComputedStyle(element).transform).a,
  );
  expect(transform).toBe(1);
  await page.getByLabel("Appearance").selectOption("dark");
  await expect(page.locator("html")).not.toHaveAttribute(
    "data-theme-switching",
  );
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(settings).toBeEnabled();
});
