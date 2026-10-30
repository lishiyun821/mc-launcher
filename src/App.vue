<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { onTaskUpdated } from "@/services/events";
import { getConfig, listTasks } from "@/services/ipc";
import { useI18n } from "@/i18n";
import type { TaskInfo } from "@/types";
import HomeView from "@/views/HomeView.vue";
import VersionsView from "@/views/VersionsView.vue";
import InstancesView from "@/views/InstancesView.vue";
import SettingsView from "@/views/SettingsView.vue";
import TaskBar from "@/components/TaskBar.vue";

type Page = "home" | "versions" | "instances" | "settings";

const { t, setLang } = useI18n();

const page = ref<Page>("home");
const tasks = ref<TaskInfo[]>([]);
let unlisten: UnlistenFn | null = null;

const nav = computed(() => [
  { key: "home" as Page, label: t("nav.home"), hint: t("nav.homeHint") },
  { key: "versions" as Page, label: t("nav.versions"), hint: t("nav.versionsHint") },
  { key: "instances" as Page, label: t("nav.instances"), hint: t("nav.instancesHint") },
  { key: "settings" as Page, label: t("nav.settings"), hint: t("nav.settingsHint") },
]);

const activeTask = computed(
  () => tasks.value.find((t) => t.state === "running") ?? null,
);
const hasTasks = computed(() => tasks.value.length > 0);

onMounted(async () => {
  // 启动时应用已保存的语言
  const cfg = await getConfig().catch(() => null);
  if (cfg?.language) setLang(cfg.language as "zh-CN" | "en");

  tasks.value = await listTasks().catch(() => []);
  unlisten = await onTaskUpdated((task) => {
    const i = tasks.value.findIndex((t) => t.id === task.id);
    if (i >= 0) tasks.value[i] = task;
    else tasks.value.unshift(task);
  });
});

onUnmounted(() => unlisten?.());
</script>

<template>
  <div class="shell">
    <aside class="sidebar">
      <div class="brand">
        <div class="logo" aria-hidden="true">
          <i v-for="n in 9" :key="n"></i>
        </div>
        <div class="brand-text">
          <div class="name">MC Launcher</div>
          <div class="ver mono">v0.1.0</div>
        </div>
      </div>

      <nav>
        <button
          v-for="item in nav"
          :key="item.key"
          class="nav-btn"
          :class="{ on: page === item.key }"
          @click="page = item.key"
        >
          <span class="dot"></span>
          <span class="label">{{ item.label }}</span>
          <span class="hint">{{ item.hint }}</span>
        </button>
      </nav>

      <div class="spacer"></div>

      <div class="status">
        <div class="status-row">
          <span class="k">{{ t("status.label") }}</span>
          <span class="v">{{ activeTask ? t("status.running") : t("status.idle") }}</span>
        </div>
        <div class="status-row">
          <span class="k">{{ t("status.tasks") }}</span>
          <span class="v mono">{{ tasks.length }}</span>
        </div>
      </div>
    </aside>

    <main class="main">
      <HomeView v-if="page === 'home'" @goto="page = $event" />
      <VersionsView v-else-if="page === 'versions'" />
      <InstancesView v-else-if="page === 'instances'" />
      <SettingsView v-else />
    </main>

    <TaskBar v-if="hasTasks" :tasks="tasks" />
  </div>
</template>

<style scoped>
.shell {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: 208px 1fr;
  grid-template-rows: 1fr auto;
  height: 100%;
}

/* ── 侧栏 ── */
.sidebar {
  grid-row: 1 / 3;
  display: flex;
  flex-direction: column;
  padding: 22px 14px 16px;
  border-right: 1px solid var(--line-2);
  background: var(--card);
  backdrop-filter: blur(24px) saturate(180%);
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 8px 22px;
}

.logo {
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  border-radius: 9px;
  background: var(--ink);
  padding: 5px;
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  grid-template-rows: repeat(3, 1fr);
  gap: 2px;
  box-shadow:
    0 1px 2px rgba(0, 0, 0, 0.12),
    inset 0 0 0 1px rgba(255, 255, 255, 0.06);
}

.logo i {
  display: block;
  border-radius: 1.5px;
  background: var(--bg);
}

/* 棋盘格：偶数格降低不透明 → 方块感 */
.logo i:nth-child(2n) {
  opacity: 0.5;
}

.brand-text .name {
  font-size: 13.5px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.brand-text .ver {
  font-size: 10px;
  color: var(--ink-3);
  margin-top: 1px;
}

nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-btn {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  text-align: left;
  transition: background 0.15s ease;
}

.nav-btn:hover {
  background: rgba(0, 0, 0, 0.035);
}

.nav-btn.on {
  background: rgba(0, 0, 0, 0.06);
}

.nav-btn .dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: transparent;
  transition: background 0.15s;
}

.nav-btn.on .dot {
  background: var(--ink);
}

.nav-btn .label {
  font-size: 13px;
  font-weight: 500;
}

.nav-btn .hint {
  margin-left: auto;
  font-size: 10px;
  color: var(--ink-4);
}

.spacer {
  flex: 1;
}

.status {
  border-top: 1px solid var(--line-2);
  padding-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.status-row {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--ink-3);
}

.status-row .v {
  color: var(--ink-2);
}

/* ── 主区 ── */
.main {
  overflow: hidden;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
</style>
