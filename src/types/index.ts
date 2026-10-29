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
}
