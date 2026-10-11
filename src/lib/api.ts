export type { GithubStatus } from "./generated/GithubStatus";
export type { GithubVerification } from "./generated/GithubVerification";
export type { PullRequestQuery } from "./generated/PullRequestQuery";
export type { PullRequest } from "./generated/PullRequest";
export type { PullRequestPage } from "./generated/PullRequestPage";
export type { GithubItem } from "./generated/GithubItem";
export type { GithubSection } from "./generated/GithubSection";
export type { PreviewRequest } from "./generated/PreviewRequest";
export type { PreviewResult } from "./generated/PreviewResult";
export type { RebaseStep } from "./generated/RebaseStep";
export type { RebasePlan } from "./generated/RebasePlan";
export type { RebaseRange } from "./generated/RebaseRange";
export type { SelectionDiff } from "./generated/SelectionDiff";
export type { LineSelection } from "./generated/LineSelection";
import type { CommitGroup } from "./generated/CommitGroup";
export type { CommitGroup } from "./generated/CommitGroup";
export type { Worktree } from "./generated/Worktree";
export type { FileHistoryEntry } from "./generated/FileHistoryEntry";
export type { FileRevision } from "./generated/FileRevision";
export type { BlameLine } from "./generated/BlameLine";
import type { AiSettings } from "./generated/AiSettings";
export type { AiSettings } from "./generated/AiSettings";
export type { AiContext } from "./generated/AiContext";
export type { AiDraft } from "./generated/AiDraft";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export type { Action } from "./generated/Action";
export type { Snapshot } from "./generated/Snapshot";
export type { Commit } from "./generated/Commit";
export type { FileChange } from "./generated/FileChange";
export type { Diff } from "./generated/Diff";
export type { Conflict } from "./generated/Conflict";
export type { GitError } from "./generated/GitError";
export type { Diagnostic } from "./generated/Diagnostic";
export type { ReflogEntry } from "./generated/ReflogEntry";
import type { Snapshot, GitError } from "./api";
export interface Opened {
  id: string;
  snapshot: Snapshot;
  trusted: boolean;
}
export interface OperationEvent {
  id: string;
  repoId: string;
  kind: string;
  status: "running" | "completed" | "failed";
  output: string;
  error: GitError | null;
  removedPath?: string | null;
}
export interface JournalEntry {
  id: string;
  repoId: string;
  kind: string;
  status: string;
  timestamp: number;
}
export interface Preferences {
  commitGroups: CommitGroup[];
  recent: string[];
  favorites: string[];
  theme: string;
  colorTheme: string;
  ai: AiSettings;
  sidebarWidth: number | null;
  detailWidth: number | null;
  journal: JournalEntry[];
}
export interface AuthPrompt {
  id: string;
  prompt: string;
}
export const desktop = isTauri();
export async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!desktop)
    throw {
      code: "desktopRequired",
      message: "Open Git Toys as a desktop app to access local repositories.",
      detail: "Run pnpm tauri dev. Browser preview has no filesystem access.",
    } satisfies GitError;
  return invoke<T>(command, args);
}
export async function onEvent<T>(name: string, callback: (payload: T) => void) {
  return desktop
    ? listen<T>(name, (event) => callback(event.payload))
    : () => {};
}
export function asError(value: unknown): GitError {
  if (value && typeof value === "object" && "message" in value)
    return {
      code: "code" in value ? String(value.code) : "error",
      message: String(value.message),
      detail: "detail" in value ? String(value.detail) : "",
    };
  return { code: "error", message: String(value), detail: "" };
}

export const defaultAiSettings = (): AiSettings => ({
  enabled: false,
  provider: "ollama",
  model: "",
  ollamaUrl: "http://127.0.0.1:11434",
});
