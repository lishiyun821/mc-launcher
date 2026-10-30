<script setup lang="ts">
import { computed, ref } from "vue";
import type { TaskInfo } from "@/types";
import { useI18n } from "@/i18n";
import { cancelTask } from "@/services/ipc";

const props = defineProps<{ tasks: TaskInfo[] }>();

const { t } = useI18n();

const expanded = ref(false);

const stateLabel = computed<Record<string, string>>(() => ({
  waiting: t("task.waiting"),
  running: t("task.running"),
  finished: t("task.finished"),
  failed: t("task.failed"),
  cancelled: t("task.cancelled"),
}));

/** 折叠时只显示还在跑的和失败的；展开时全部 */
const shown = computed(() => {
  const all = props.tasks;
  const list = expanded.value
    ? all
    : all.filter((x) => x.state === "running" || x.state === "failed" || x.state === "waiting");
  return list.slice(0, expanded.value ? 30 : 4);
});

const runningCount = computed(
  () => props.tasks.filter((x) => x.state === "running" || x.state === "waiting").length,
);

function fmtElapsed(task: TaskInfo) {
  if (!task.startedAt) return "";
  const end = task.finishedAt ?? Date.now();
  const sec = Math.max(0, Math.round((end - task.startedAt) / 1000));
  if (sec < 60) return sec + "s";
  return Math.floor(sec / 60) + "m" + (sec % 60) + "s";
}

async function cancel(id: string) {
  await cancelTask(id).catch(() => {});
}
</script>

<template>
  <div class="taskbar">
    <div class="head">
      <button class="toggle" @click="expanded = !expanded">
        <span class="chev" :class="{ open: expanded }">›</span>
        <span class="title">{{ t("task.title") }}</span>
      </button>
      <span class="count mono">{{ props.tasks.length }}</span>
      <span v-if="runningCount" class="running mono">{{ runningCount }} {{ t("task.active") }}</span>
      <span class="spacer"></span>
      <button v-if="props.tasks.length" class="link" @click="expanded = !expanded">
        {{ expanded ? t("task.collapse") : t("task.expand") }}
      </button>
    </div>

    <div v-if="!shown.length" class="idle">{{ t("task.none") }}</div>

    <div v-else class="list">
      <div v-for="task in shown" :key="task.id" class="task" :class="task.state">
        <span class="dot"></span>

        <div class="col">
          <div class="line1">
            <span class="name">{{ task.title }}</span>
            <span class="pct mono">{{ Math.round(task.progress * 100) }}<small>%</small></span>
          </div>

          <div v-if="expanded" class="bar">
            <span class="fill" :style="{ width: Math.round(task.progress * 100) + '%' }"></span>
          </div>

          <div class="line2 mono">
            <span class="detail">{{ task.detail || task.error || stateLabel[task.state] }}</span>
            <span v-if="task.startedAt" class="elapsed">{{ fmtElapsed(task) }}</span>
          </div>
        </div>

        <span class="state">{{ stateLabel[task.state] ?? task.state }}</span>
        <button
          v-if="task.state === 'running' || task.state === 'waiting'"
          class="cancel"
          @click="cancel(task.id)"
        >
          {{ t("task.cancel") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.taskbar {
  grid-column: 2;
  border-top: 1px solid var(--line-2);
  background: var(--card);
  padding: 7px 16px 9px;
  max-height: 260px;
  overflow: auto;
}
.head { display: flex; align-items: center; gap: 8px; margin-bottom: 3px; }
.toggle { display: flex; align-items: center; gap: 4px; padding: 0; background: none; border: none; cursor: pointer; color: inherit; }
.chev { font-size: 11px; color: var(--ink-4); transition: transform 0.15s; display: inline-block; }
.chev.open { transform: rotate(90deg); }
.head .title { font-size: 10px; letter-spacing: 0.14em; color: var(--ink-3); text-transform: uppercase; }
.head .count { font-size: 10px; color: var(--ink-4); }
.running { font-size: 9.5px; color: var(--warn); }
.spacer { flex: 1; }
.link { font-size: 10px; color: var(--ink-3); background: none; border: none; cursor: pointer; padding: 0; }
.link:hover { color: var(--ink); }
.idle { font-size: 10.5px; color: var(--ink-4); padding: 3px 2px 5px; }
.list { display: flex; flex-direction: column; gap: 2px; }
.task { display: flex; align-items: center; gap: 9px; padding: 3px 2px; font-size: 11.5px; }
.dot { width: 5px; height: 5px; border-radius: 50%; background: var(--ink-4); flex-shrink: 0; }
.task.running .dot { background: var(--warn); animation: pulse 1.2s ease-in-out infinite; }
.task.finished .dot { background: var(--ok); }
.task.failed .dot { background: var(--err); }
.task.cancelled .dot { background: var(--ink-4); }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
.col { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 3px; }
.line1 { display: flex; align-items: baseline; gap: 8px; }
.name { font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; color: var(--ink); }
.pct { font-size: 10.5px; color: var(--ink-2); flex-shrink: 0; }
.bar { height: 3px; border-radius: 2px; background: var(--line); overflow: hidden; }
.fill { display: block; height: 100%; background: var(--ink-2); transition: width 320ms var(--ease-apple), background-color 220ms var(--ease-apple); }
.line2 { display: flex; align-items: baseline; gap: 8px; font-size: 10px; color: var(--ink-3); }
.detail { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.elapsed { flex-shrink: 0; opacity: 0.7; }
.state { font-size: 10px; color: var(--ink-3); min-width: 34px; text-align: right; flex-shrink: 0; }
.cancel { padding: 2px 8px; font-size: 10px; border: 1px solid var(--line); border-radius: 6px; background: transparent; color: var(--err); cursor: pointer; flex-shrink: 0; }
.cancel:hover { background: rgba(201, 48, 44, 0.08); }
</style>
