use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::Emitter;
use tokio::sync::RwLock;

/* ═══════════════ 数据结构 ═══════════════ */

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskState {
    Waiting,
    Running,
    Finished,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub id: String,
    pub title: String,
    pub state: TaskState,
    pub progress: f32,
    pub detail: String,
    pub error: Option<String>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskEvent {
    pub task: TaskInfo,
}

pub const EVT_TASK: &str = "task-updated";

/* ═══════════════ 任务管理器 ═══════════════ */

pub struct TaskManager {
    app: tauri::AppHandle,
    inner: Arc<RwLock<HashMap<String, TaskInfo>>>,
    cancels: Arc<RwLock<HashMap<String, Arc<AtomicBool>>>>,
}

impl TaskManager {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            inner: Arc::new(RwLock::new(HashMap::new())),
            cancels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建一个任务，返回可操作的句柄
    pub fn create(self: &Arc<Self>, title: impl Into<String>) -> TaskHandle {
        let id = uuid::Uuid::new_v4().to_string();
        let info = TaskInfo {
            id: id.clone(),
            title: title.into(),
            state: TaskState::Running,
            progress: 0.0,
            detail: "准备中".to_string(),
            error: None,
            started_at: Some(now_ms()),
            finished_at: None,
        };
        let cancel = Arc::new(AtomicBool::new(false));

        let mgr = self.clone();
        let info_clone = info.clone();
        let cancel_clone = cancel.clone();
        // 同步插入（用 block_in_place 不合适，改用 tokio::spawn + try_write）
        // 简单起见：用 try_write 循环，拿不到就 spawn 异步插入
        if let Ok(mut map) = self.inner.try_write() {
            map.insert(id.clone(), info_clone);
        } else {
            let inner = self.inner.clone();
            tokio::spawn(async move {
                inner.write().await.insert(info_clone.id.clone(), info_clone);
            });
        }
        if let Ok(mut map) = self.cancels.try_write() {
            map.insert(id.clone(), cancel_clone);
        }

        // 立刻推一次，前端能马上看到
        let _ = self.app.emit(EVT_TASK, TaskEvent { task: info });

        TaskHandle {
            mgr,
            id,
            cancel,
        }
    }

    pub async fn list(&self) -> Vec<TaskInfo> {
        let map = self.inner.read().await;
        let mut v: Vec<_> = map.values().cloned().collect();
        v.sort_by_key(|t| std::cmp::Reverse(t.started_at.unwrap_or(0)));
        v
    }

    pub async fn cancel(&self, id: &str) {
        let map = self.cancels.read().await;
        if let Some(flag) = map.get(id) {
            flag.store(true, Ordering::Relaxed);
        }
    }

    async fn patch<F>(&self, id: &str, f: F)
    where
        F: FnOnce(&mut TaskInfo),
    {
        let snapshot = {
            let mut map = self.inner.write().await;
            match map.get_mut(id) {
                Some(t) => {
                    f(t);
                    Some(t.clone())
                }
                None => None,
            }
        };
        if let Some(task) = snapshot {
            let _ = self.app.emit(EVT_TASK, TaskEvent { task });
        }
    }
}

/* ═══════════════ 任务句柄 ═══════════════ */

/// 任务句柄（可克隆：克隆出来的句柄操作同一个任务，方便丢进 tokio::spawn）
#[derive(Clone)]
pub struct TaskHandle {
    mgr: Arc<TaskManager>,
    id: String,
    cancel: Arc<AtomicBool>,
}

impl TaskHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 更新进度（0..1）与状态文本
    pub async fn progress(&self, p: f32, detail: impl Into<String>) {
        let p = p.clamp(0.0, 1.0);
        let detail = detail.into();
        self.mgr
            .patch(&self.id, move |t| {
                t.progress = p;
                t.detail = detail;
                t.state = TaskState::Running;
            })
            .await;
    }

    /// 同步版进度更新：给「不能 await 的同步回调」用（如下载进度回调）
    pub fn progress_sync(&self, p: f32, detail: impl Into<String>) {
        let p = p.clamp(0.0, 1.0);
        let detail = detail.into();

        let Ok(mut map) = self.mgr.inner.try_write() else {
            return;
        };
        let Some(t) = map.get_mut(&self.id) else {
            return;
        };
        t.progress = p;
        t.detail = detail;
        t.state = TaskState::Running;
        let snapshot = t.clone();
        drop(map);

        let _ = self.mgr.app.emit(EVT_TASK, TaskEvent { task: snapshot });
    }

    pub async fn finish(&self, detail: impl Into<String>) {
        let detail = detail.into();
        self.mgr
            .patch(&self.id, move |t| {
                t.progress = 1.0;
                t.detail = detail;
                t.state = TaskState::Finished;
                t.finished_at = Some(now_ms());
            })
            .await;
    }

    pub async fn fail(&self, err: impl Into<String>) {
        let err = err.into();
        self.mgr
            .patch(&self.id, move |t| {
                t.state = TaskState::Failed;
                t.error = Some(err.clone());
                t.detail = err;
                t.finished_at = Some(now_ms());
            })
            .await;
    }

    pub async fn cancelled(&self) {
        self.mgr
            .patch(&self.id, |t| {
                t.state = TaskState::Cancelled;
                t.detail = "已取消".to_string();
                t.finished_at = Some(now_ms());
            })
            .await;
    }

    /// 供长任务轮询：用户是否点了取消
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// 如果已取消，返回 Err 方便 `?` 提前中断
    pub fn check_cancel(&self) -> Result<(), anyhow::Error> {
        if self.is_cancelled() {
            Err(anyhow::anyhow!("任务已取消"))
        } else {
            Ok(())
        }
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
