<script setup lang="ts">
import type { TaskInfo } from "@/types";

defineProps<{ tasks: TaskInfo[] }>();

const stateLabel: Record<string, string> = {
  waiting: "等待",
  running: "进行中",
  finished: "完成",
  failed: "失败",
  cancelled: "已取消",
};
</script>

<template>
  <div class="taskbar">
    <div class="head">
      <span class="title">任务</span>
      <span class="count mono">{{ tasks.length }}</span>
    </div>
    <div class="list">
      <div
        v-for="t in tasks.slice(0, 6)"
        :key="t.id"
        class="task"
        :class="t.state"
      >
        <span class="dot"></span>
        <span class="name">{{ t.title }}</span>
        <span class="detail">{{ t.detail }}</span>
        <span class="pct mono">{{ Math.round(t.progress * 100) }}<small>%</small></span>
        <span class="state">{{ stateLabel[t.state] ?? t.state }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.taskbar {
  grid-column: 2;
  border-top: 1px solid var(--line-2);
  background: var(--card);
  backdrop-filter: blur(24px) saturate(180%);
  padding: 8px 16px 10px;
  max-height: 168px;
  overflow: auto;
}

.head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 5px;
}

.head .title {
  font-size: 10px;
  letter-spacing: 0.14em;
  color: var(--ink-3);
  text-transform: uppercase;
}

.head .count {
  font-size: 10px;
  color: var(--ink-4);
}

.list {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.task {
  display: grid;
  grid-template-columns: 6px 1fr auto auto auto;
  align-items: center;
  gap: 9px;
  padding: 4px 2px;
  font-size: 11.5px;
}

.dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--ink-4);
}

.task.running .dot {
  background: var(--warn);
  animation: pulse 1.2s ease-in-out infinite;
}
.task.finished .dot {
  background: var(--ok);
}
.task.failed .dot {
  background: var(--err);
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.name {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.detail {
  color: var(--ink-3);
  font-size: 10.5px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 320px;
}

.pct {
  color: var(--ink-2);
  font-size: 11px;
}

.state {
  font-size: 10px;
  color: var(--ink-3);
  min-width: 34px;
  text-align: right;
}
</style>
