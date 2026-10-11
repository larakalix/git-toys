export function githubRepository(url: string): string | null {
  const match =
    /^(?:https:\/\/github\.com\/|git@github\.com:|ssh:\/\/git@github\.com\/)([^/?#]+)\/([^/?#]+)\/?$/.exec(
      url,
    );
  if (!match) return null;
  const owner = match[1]!,
    repo = match[2]!.replace(/\.git$/, "");
  return [owner, repo].every(
    (v) => v !== "." && v !== ".." && /^[A-Za-z0-9_.-]+$/.test(v),
  )
    ? `${owner}/${repo}`
    : null;
}
