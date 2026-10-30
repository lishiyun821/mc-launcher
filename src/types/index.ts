/**
 * 与 Rust 侧共享的类型定义
 * 改动这里必须同步 src-tauri/src/core/models.rs
 */

/** 版本清单里的一个版本 */
export interface VersionSummary {
  id: string;
  type: string; // release | snapshot | old_beta | old_alpha
  url: string;
  releaseTime: string;
  /** 是否为当前最新正式版 */
  isLatest: boolean;
}

/** version_manifest 的解析结果 */
export interface VersionManifest {
  latest: {
    release: string;
    snapshot: string;
  };
  versions: VersionSummary[];
}

/** 任务状态（Phase 0 的任务调度器） */
export type TaskState = "waiting" | "running" | "finished" | "failed" | "cancelled";

export interface TaskInfo {
  id: string;
  title: string;
  state: TaskState;
  /** 0..1 */
  progress: number;
  /** 当前步骤描述 */
  detail: string;
  error?: string;
  startedAt?: number;
  finishedAt?: number;
}

/** 从 Rust 推送来的任务事件 */
export interface TaskEvent {
  task: TaskInfo;
}

/** 下载源偏好 */
export type DownloadSource = "official" | "mirror";

/** 界面语言 */
export type Lang = "zh-CN" | "en";

export interface AppConfig {
  downloadSource: DownloadSource;
  language: Lang;
  /** 离线账户名字 */
  playerName: string;
}

/** 启动结果 */
export interface LaunchReport {
  pid: number;
  version: string;
  mainClass: string;
  java: string;
  javaMajor: number;
  account: string;
  uuid: string;
  classpathCount: number;
  memoryMb: number;
  commandPreview: string;
  gameDir: string;
  missing: string[];
}

/** 安装结果报告 */
export interface InstallReport {
  id: string;
  mainClass: string;
  libraryCount: number;
  platformLibraryCount: number;
  nativeCount: number;
  /** 实际解压出的 native 文件数 */
  nativeExtracted: number;
  assetCount: number;
  assetIndexId: string;
  clientSize: number;
  /** 继承链，父 → 子 */
  chain: string[];
  jsonPath: string;
  gameDir: string;
  /** 本次真正下载的文件数 */
  downloaded: number;
  /** 已存在跳过（校验通过）的文件数 */
  skipped: number;
  /** 失败的文件 */
  failed: string[];
  totalBytes: number;
  elapsedMs: number;
}
