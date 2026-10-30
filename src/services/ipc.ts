import type { UpdateInfo } from "@/types";
export type { UpdateInfo };
import { invoke } from "@tauri-apps/api/core";

/**
 * IPC 统一入口 —— 所有 Rust 命令都从这里走
 * 好处：只有这一层知道命令名，方便统一错误处理 / 日志 / 埋点
 */

/** 统一错误处理：把 Rust 的 Err(String) 转成 JS Error */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const msg = typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
    throw new Error(`[${cmd}] ${msg}`);
  }
}

/* ───────── 应用 ───────── */

export const getAppVersion = () => call<string>("app_version");

export const getConfig = () => call<import("@/types").AppConfig>("get_config");

export const setConfig = (config: import("@/types").AppConfig) =>
  call<void>("set_config", { config });

/* ───────── 版本 ───────── */

export const fetchVersionManifest = (forceRefresh = false) =>
  call<import("@/types").VersionManifest>("fetch_version_manifest", { forceRefresh });

export const listLocalVersions = () => call<string[]>("list_local_versions");

/* ───────── 安装 ───────── */

export const installVersion = (id: string) =>
  call<import("@/types").InstallReport>("install_version", { id });

/* ───────── 启动 ───────── */

export const launchVersion = (id: string, accountName: string, memoryMb?: number) =>
  call<import("@/types").LaunchReport>("launch_version", {
    id,
    accountName,
    memoryMb: memoryMb ?? null,
  });

/* ───────── 实例 ───────── */

export const listInstances = () =>
  call<import("@/types").InstanceInfo[]>("list_instances");

export const getInstance = (id: string) =>
  call<import("@/types").InstanceConfig>("get_instance", { id });

export const updateInstance = (
  id: string,
  config: import("@/types").InstanceConfig
) => call<void>("update_instance", { id, config });

export const deleteInstance = (id: string) =>
  call<number>("delete_instance", { id });

export const duplicateInstance = (from: string, to: string) =>
  call<void>("duplicate_instance", { from, to });

export const openInstanceFolder = (id: string) =>
  call<string>("open_instance_folder", { id });

export const openGameFolder = () => call<string>("open_game_folder");

export const readInstanceLog = (id: string, lines?: number) =>
  call<import("@/types").LogView>("read_instance_log", { id, lines: lines ?? null });

export const verifyInstance = (id: string) =>
  call<import("@/types").VerifyReport>("verify_instance", { id });

/* ───────── 账户 ───────── */

export const listAccounts = () =>
  call<import("@/types").AccountsView>("list_accounts");

export const msLoginStart = () =>
  call<import("@/types").DeviceCode>("ms_login_start");

export const msLoginPoll = (deviceCode: string) =>
  call<import("@/types").LoginState>("ms_login_poll", { deviceCode });

export const removeAccount = (id: string) =>
  call<import("@/types").AccountsView>("remove_account", { id });

export const setActiveAccount = (id: string) =>
  call<import("@/types").AccountsView>("set_active_account", { id });

export const refreshAccount = (id: string) =>
  call<import("@/types").AccountInfo>("refresh_account", { id });

/* ───────── 任务 ───────── */

export const listTasks = () => call<import("@/types").TaskInfo[]>("list_tasks");

export const cancelTask = (taskId: string) => call<void>("cancel_task", { taskId });

/* ───────── 系统 ───────── */

export const scanJava = () => call<JavaInfo[]>("scan_java");

/* ───────── Java 运行时 ───────── */

export interface RuntimeInfo {
  major: number;
  path: string;
}

export interface JreInfo {
  major: number;
  version: string;
  url: string;
  size: number;
  name: string;
}

export const listJavaRuntimes = () =>
  call<RuntimeInfo[]>("list_java_runtimes");

export const javaRemoteInfo = (major: number) =>
  call<JreInfo>("java_remote_info", { major });

export const installJava = (major: number) =>
  call<string>("install_java", { major });

/* ───────── 更新检查 ───────── */

export const checkUpdate = () => call<UpdateInfo>("check_update");

export interface JavaInfo {
  path: string;
  version: string;
  major: number;
  vendor: string;
  is64Bit: boolean;
}
