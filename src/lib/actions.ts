import type { Action } from "./api";
export type ActionKind = Action["kind"];
interface Field {
  key: string;
  label: string;
  type?: "checkbox" | "number" | "textarea";
  options?: string[];
  optional?: boolean;
  placeholder?: string;
}
export interface ActionDefinition {
  kind: ActionKind;
  title: string;
  description: string;
  danger?: boolean;
  fields: Field[];
}
const field = (
  key: string,
  label: string,
  extra: Partial<Field> = {},
): Field => ({ key, label, ...extra });
const target = field("target", "Target commit or branch", {
  placeholder: "main, a tag, or a commit hash",
});
const name = field("name", "Name");
const remote = field("remote", "Remote", { placeholder: "origin" });
export const actionDefinitions: ActionDefinition[] = [
  {
    kind: "createBranch",
    title: "Create branch",
    description:
      "Create a branch at the selected starting point. Your current branch stays checked out.",
    fields: [
      name,
      field("start", "Starting point", { optional: true, placeholder: "HEAD" }),
    ],
  },
  {
    kind: "switchBranch",
    title: "Switch branch",
    description:
      "Check out a local branch. Git will refuse changes that would overwrite your work.",
    fields: [name],
  },
  {
    kind: "renameBranch",
    title: "Rename branch",
    description: "Rename a local branch. Remote branches are not renamed.",
    fields: [field("old", "Current name"), name],
  },
  {
    kind: "deleteBranch",
    title: "Delete branch",
    description:
      "Delete a local branch. Force deletion may leave commits reachable only through recovery references.",
    danger: true,
    fields: [
      name,
      field("force", "Force delete an unmerged branch", { type: "checkbox" }),
    ],
  },
  {
    kind: "setUpstream",
    title: "Set upstream",
    description:
      "Change the remote-tracking branch used for pull and ahead/behind counts.",
    fields: [
      field("branch", "Local branch"),
      field("upstream", "Upstream branch", { placeholder: "origin/main" }),
    ],
  },
  {
    kind: "createTag",
    title: "Create tag",
    description:
      "Create a lightweight local tag. This does not push it to a remote.",
    fields: [name, target],
  },
  {
    kind: "deleteTag",
    title: "Delete tag",
    description: "Remove a local tag. The remote tag is unchanged.",
    danger: true,
    fields: [name],
  },
  {
    kind: "addRemote",
    title: "Add remote",
    description:
      "Configure a repository URL. Credentials stay with your existing Git helpers.",
    fields: [name, field("url", "Repository URL")],
  },
  {
    kind: "editRemote",
    title: "Edit remote",
    description: "Change the URL for a configured remote.",
    fields: [name, field("url", "Repository URL")],
  },
  {
    kind: "removeRemote",
    title: "Remove remote",
    description:
      "Remove this remote and its tracking references from local configuration.",
    danger: true,
    fields: [name],
  },
  {
    kind: "fetch",
    title: "Fetch",
    description:
      "Download remote objects and update remote-tracking branches. Your working files stay unchanged.",
    fields: [remote],
  },
  {
    kind: "pull",
    title: "Pull changes",
    description:
      "Fetch and integrate the current branch’s upstream. Choose an explicit strategy. Your changes will never be automatically stashed.",
    fields: [
      remote,
      field("strategy", "Integration strategy", {
        options: ["ff-only", "merge", "rebase"],
      }),
    ],
  },
  {
    kind: "push",
    title: "Push changes",
    description:
      "Push the current branch to the same branch name on the selected remote. Force-with-lease rewrites remote history only if its tip matches your last fetch.",
    fields: [
      remote,
      field("publish", "Set upstream tracking for this branch", {
        type: "checkbox",
      }),
      field("force", "Use force-with-lease (rewrites remote history)", {
        type: "checkbox",
      }),
    ],
  },
  {
    kind: "merge",
    title: "Merge into current branch",
    description:
      "Integrate the target into your current branch. Conflicts pause the operation for review.",
    fields: [target],
  },
  {
    kind: "rebase",
    title: "Rebase current branch",
    description:
      "Replay current branch commits onto the target. Commit hashes change; published history may require force-with-lease.",
    danger: true,
    fields: [target],
  },
  {
    kind: "cherryPick",
    title: "Cherry-pick commit",
    description:
      "Apply the selected commit as a new commit on your current branch.",
    fields: [target],
  },
  {
    kind: "revert",
    title: "Revert commit",
    description:
      "Create a new commit that reverses the selected commit. Merge commits require terminal options in this version.",
    fields: [target],
  },
  {
    kind: "reset",
    title: "Reset current branch",
    description:
      "Move the current branch to the target. Soft keeps changes staged; mixed unstages them; hard destroys tracked uncommitted changes.",
    danger: true,
    fields: [
      target,
      field("mode", "Reset mode", { options: ["soft", "mixed", "hard"] }),
    ],
  },
  {
    kind: "stash",
    title: "Stash working changes",
    description:
      "Save tracked and untracked working changes in a stash, then clean the working copy. Ignored files are excluded.",
    fields: [field("message", "Stash message", { optional: true })],
  },
  {
    kind: "applyStash",
    title: "Apply stash",
    description:
      "Apply a saved stash. Pop also removes it after successful application.",
    fields: [
      field("index", "Stash index", { type: "number" }),
      field("pop", "Pop after applying", { type: "checkbox" }),
    ],
  },
  {
    kind: "dropStash",
    title: "Drop stash",
    description:
      "Delete a saved stash. Uncommitted work stored only here may be lost.",
    danger: true,
    fields: [field("index", "Stash index", { type: "number" })],
  },
  {
    kind: "continue",
    title: "Continue operation",
    description:
      "Continue the active operation after resolving and staging conflicts.",
    fields: [],
  },
  {
    kind: "abort",
    title: "Abort operation",
    description:
      "Ask Git to return to the state before the operation. Edits made during conflict resolution may be lost.",
    danger: true,
    fields: [],
  },
  {
    kind: "discard",
    title: "Discard unstaged changes",
    description:
      "Restore the selected tracked file from the index. These unstaged edits are not recoverable through reflog.",
    danger: true,
    fields: [],
  },
  {
    kind: "commit",
    title: "Commit changes",
    description:
      "Record staged changes. Amend replaces the current commit and changes its hash.",
    fields: [
      field("message", "Commit message", { type: "textarea" }),
      field("amend", "Amend the current commit", { type: "checkbox" }),
    ],
  },
];
export function actionDefinition(kind: string) {
  return actionDefinitions.find((a) => a.kind === kind);
}

export function operationSuccess(kind: string): string {
  const messages: Record<string, string> = {
    addWorktree: "Workspace created",
    removeWorktree: "Workspace removed; branch preserved",
    restoreFileVersion: "File restored as an unstaged change",
    push: "Changes pushed",
    fetch: "Remote references fetched",
    pull: "Branch updated",
    applyPreview: "Reviewed Git operation completed",
    interactiveRebase: "History rewritten; recovery branch preserved",
    checkoutPullRequest: "Pull request checked out",
    stageGroup: "Commit group staged",
    stage: "Changes staged",
    unstage: "Changes unstaged",
    stageHunk: "Hunk staged",
    unstageHunk: "Hunk unstaged",
    commit: "Commit created",
    createBranch: "Branch created",
    switchBranch: "Branch switched",
    renameBranch: "Branch renamed",
    deleteBranch: "Branch deleted",
    setUpstream: "Upstream updated",
    createTag: "Tag created",
    deleteTag: "Tag deleted",
    addRemote: "Remote added",
    editRemote: "Remote updated",
    removeRemote: "Remote removed",
    merge: "Merge completed",
    rebase: "Rebase completed",
    cherryPick: "Commit cherry-picked",
    revert: "Commit reverted",
    reset: "Branch reset",
    stash: "Changes stashed",
    applyStash: "Stash applied",
    dropStash: "Stash dropped",
    resolve: "File marked resolved",
    continue: "Git operation continued",
    abort: "Git operation aborted",
    discard: "Changes discarded",
  };
  return messages[kind] ?? `${actionDefinition(kind)?.title ?? kind} completed`;
}
