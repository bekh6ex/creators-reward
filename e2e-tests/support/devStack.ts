import * as fs from 'fs';
import * as path from 'path';

export const repoRoot = path.resolve(__dirname, '../..');

interface DevStackUrls {
  website1: string;
  website2: string;
  app: string;
  creator: string;
  api: string;
}

export interface DevStack {
  urls: DevStackUrls;
  env: Record<string, string>;
}

/**
 * The running worktree-dev stack: its public URLs (`urls.json`) and the ports and settings
 * it was started with (`env`). See `.agents/skills/worktree-dev/SKILL.md`.
 */
export function devStack(): DevStack {
  const stateDir = path.join(repoRoot, 'dev/.state');
  let slot: string;
  try {
    slot = fs.readFileSync(path.join(stateDir, 'current-slot'), 'utf8').trim();
  } catch {
    throw new Error('No dev stack found: start it first (`./dev/start.sh --detach --wait-ready`).');
  }
  const urls = JSON.parse(
    fs.readFileSync(path.join(stateDir, slot, 'urls.json'), 'utf8'),
  ) as DevStackUrls;
  const env: Record<string, string> = {};
  for (const line of fs.readFileSync(path.join(stateDir, slot, 'env'), 'utf8').split('\n')) {
    const separator = line.indexOf('=');
    if (separator > 0) env[line.slice(0, separator)] = line.slice(separator + 1);
  }
  return { urls, env };
}

/** The public URL of a dev-stack host, e.g. `website1.crlocal` -> `https://website1.crlocal:18090`. */
export function urlOf(stack: DevStack, host: string): string {
  const url = Object.values(stack.urls).find((candidate) => new URL(candidate).hostname === host);
  if (!url) throw new Error(`The dev stack serves no host named ${host}`);
  return url;
}
