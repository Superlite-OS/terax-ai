import { invoke } from "@tauri-apps/api/core";

let cachedDir: string | undefined;
let cachedTaskManager = false;

export async function initLaunchDir(): Promise<void> {
  const dir =
    (await invoke<string | null>("get_launch_dir").catch(() => null)) ??
    (await invoke<string>("workspace_current_dir").catch(() => null));
  cachedDir = dir ? dir.replace(/\\/g, "/") : undefined;
  cachedTaskManager = await invoke<boolean>("get_open_task_manager").catch(() => false);
}

export function getLaunchDir(): string | undefined {
  return cachedDir;
}

export function getOpenTaskManager(): boolean {
  return cachedTaskManager;
}
