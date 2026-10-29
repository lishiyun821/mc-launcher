import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TaskEvent, TaskInfo } from "@/types";

/**
 * 事件总线（前端侧）
 * Rust 通过 app.emit() 推事件，这里统一订阅并分发给组件
 */

export const EVT_TASK = "task-updated";
export const EVT_LOG = "app-log";

type Handler<T> = (payload: T) => void;

/** 订阅任务更新 */
export function onTaskUpdated(handler: Handler<TaskInfo>): Promise<UnlistenFn> {
  return listen<TaskEvent>(EVT_TASK, (e) => handler(e.payload.task));
}

/** 订阅日志 */
export function onLog(handler: Handler<{ level: string; message: string }>) {
  return listen<{ level: string; message: string }>(EVT_LOG, (e) => handler(e.payload));
}
