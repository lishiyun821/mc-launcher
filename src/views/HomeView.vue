<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import SkinAvatar from "@/components/SkinAvatar.vue";
import {
  getConfig,
  launchVersion,
  listAccounts,
  listInstances,
  setActiveAccount,
  setConfig,
} from "@/services/ipc";
import { useI18n } from "@/i18n";
import type { AccountInfo, InstanceInfo } from "@/types";
import LogModal from "@/components/LogModal.vue";

const emit = defineEmits<{ (e: "goto", page: string): void }>();
const { t } = useI18n();

const instances = ref<InstanceInfo[]>([]);
const activeAccount = ref<AccountInfo | null>(null);
const accounts = ref<AccountInfo[]>([]);
const playerName = ref("MyisaCN");
const activeId = ref<string | null>(null);
const loading = ref(true);
const launching = ref(false);
const report = ref("");
const showLog = ref(false);

/** 首页显示的实例：选中的 → 最近玩过的 → 第一个 */
const current = computed<InstanceInfo | null>(() => {
  const list = instances.value;
  if (!list.length) return null;
  return (
    list.find((i) => i.id === activeId.value) ??
    [...list].sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0))[0]
  );
});

/** 其它玩过的实例 */
const recent = computed(() =>
  instances.value
    .filter((i) => i.lastPlayed && i.id !== current.value?.id)
    .sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0))
    .slice(0, 4),
);

/** 当前账户类型：msa / offline / none */
const activeKind = computed(() => activeAccount.value?.kind ?? "none");

/** 顶部「正版 / 离线」切换 */
async function switchKind(kind: "msa" | "offline") {
  const target = accounts.value.find((a) => a.kind === kind);
  if (!target) {
    // 这种账户还没有 —— 去账户页加
    emit("goto", "auth");
    return;
  }
  if (target.id === activeAccount.value?.id) return;
  try {
    await setActiveAccount(target.id);
    await load();
  } catch (e) {
    report.value = e instanceof Error ? e.message : String(e);
  }
}

async function load() {
  loading.value = true;
  const [cfg, list, accs] = await Promise.all([
    getConfig().catch(() => null),
    listInstances().catch(() => [] as InstanceInfo[]),
    listAccounts().catch(() => null),
  ]);

  if (cfg) {
    playerName.value = cfg.playerName || "MyisaCN";
    activeId.value = cfg.activeInstance ?? null;
  }
  instances.value = list;
  accounts.value = accs?.accounts ?? [];
  // 优先用后端给的 active 布尔字段；退一步再按 id 匹配。
  // 账户页和首页状态必须一致，任何一边拿不到都退回"未登录"只为兜底。
  if (accs && Array.isArray(accs.accounts)) {
    activeAccount.value =
      accs.accounts.find((a) => a.active) ??
      accs.accounts.find((a) => a.id === accs.active) ??
      null;
    console.debug("[home] accounts =", accs.accounts.length, "active =", accs.active, "→", activeAccount.value?.name ?? "none");
  }
  loading.value = false;
}

onMounted(load);

async function select(id: string) {
  activeId.value = id;
  const cfg = await getConfig().catch(() => null);
  if (cfg) {
    await setConfig({ ...cfg, activeInstance: id }).catch(() => {});
  }
}

async function play() {
  const it = current.value;
  if (!it || launching.value) return;

  launching.value = true;
  report.value = "";

  try {
    const r = await launchVersion(it.id, playerName.value);
    report.value = "PID " + r.pid + " · Java " + r.javaMajor + " · " + r.classpathCount + " 项 classpath · " + r.memoryMb + " MB 堆 · " + r.account;
    setTimeout(load, 1500);
  } catch (e) {
    report.value = t("home.launchFailed") + (e instanceof Error ? e.message : String(e));
  } finally {
    launching.value = false;
  }
}

async function playInstance(id: string) {
  await select(id);
  await play();
}

function fmtAgo(sec: number | null) {
  if (!sec) return t("home.neverPlayed");
  const diff = Date.now() / 1000 - sec;
  if (diff < 60) return t("home.justNow");
  if (diff < 3600) return Math.floor(diff / 60) + " " + t("home.minutesAgo");
  if (diff < 86400) return Math.floor(diff / 3600) + " " + t("home.hoursAgo");
  return Math.floor(diff / 86400) + " " + t("home.daysAgo");
}

function kindLabel(kind: string | null) {
  if (kind === "snapshot") return t("versions.typeSnapshot");
  if (kind === "old_beta") return t("versions.typeOldBeta");
  if (kind === "old_alpha") return t("versions.typeOldAlpha");
  return t("versions.typeRelease");
}
</script>

<template>
  <div class="wrap">
    <!-- 顶部：正版 / 离线 -->
    <div class="kind-switch">
      <button class="kind" :class="{ on: activeKind === 'msa' }" @click="switchKind('msa')">
        <span class="kind-dot"></span>
        {{ t("home.msaMode") }}
      </button>
      <button class="kind" :class="{ on: activeKind === 'offline' }" @click="switchKind('offline')">
        <span class="kind-dot"></span>
        {{ t("home.offlineMode") }}
      </button>
    </div>

    <!-- 头像 + 玩家名 -->
    <div class="hero">
      <SkinAvatar
        class="hero-avatar"
        :uuid="activeAccount?.uuid"
        :name="activeAccount?.name ?? playerName"
        :skin-url="activeAccount?.skinUrl"
        :size="96"
      />
      <div class="hero-name">{{ activeAccount?.name ?? playerName }}</div>
      <div class="hero-sub mono">
        <template v-if="current">
          {{ current.name }}
          <span class="dot-sep">·</span>
          {{ kindLabel(current.kind) }}
        </template>
        <template v-else>{{ t("home.noVersion") }}</template>
      </div>
    </div>

    <!-- 启动 -->
    <div v-if="current" class="hero-actions">
      <button class="big-launch" :disabled="launching" @click="play">
        <span v-if="launching" class="spinner"></span>
        <span class="bl-main">{{ launching ? t("home.launching") : t("home.launch") }}</span>
        <span class="bl-sub mono">{{ current.name }}</span>
      </button>

      <div class="hero-row">
        <button class="hero-sq" @click="emit('goto', 'versions')">
          {{ t("home.pickVersion") }}
        </button>
        <button class="hero-sq" @click="emit('goto', 'instances')">
          {{ t("home.versionSettings") }}
        </button>
      </div>

      <div v-if="report" class="report-line">
        <span class="report mono">{{ report }}</span>
        <button class="log-link" @click="showLog = true">{{ t("home.log") }}</button>
      </div>
    </div>

    <section v-else-if="!loading" class="empty">
      <div class="empty-title">{{ t("home.noInstances") }}</div>
      <p class="empty-desc">{{ t("home.noInstancesDesc") }}</p>
      <button class="primary" @click="emit('goto', 'versions')">
        {{ t("home.goInstall") }} →
      </button>
    </section>

    <section v-if="recent.length" class="recent">
      <div class="group-title">{{ t("home.recent") }}</div>
      <div class="recent-grid">
        <button
          v-for="i in recent"
          :key="i.id"
          class="recent-item"
          @click="playInstance(i.id)"
        >
          <span class="ri-name">{{ i.name }}</span>
          <span class="ri-meta mono">{{ fmtAgo(i.lastPlayed) }}</span>
        </button>
      </div>
    </section>

    <section v-if="instances.length" class="quick">
      <button class="quick-item" @click="emit('goto', 'versions')">
        <span class="q-title">{{ t("home.quickVersions") }}</span>
        <span class="q-desc">{{ t("home.quickVersionsDesc") }}</span>
      </button>
      <button class="quick-item" @click="emit('goto', 'instances')">
        <span class="q-title">{{ t("home.quickInstances") }}</span>
        <span class="q-desc">{{ t("home.quickInstancesDesc") }}</span>
      </button>
      <button class="quick-item" @click="emit('goto', 'settings')">
        <span class="q-title">{{ t("home.quickSettings") }}</span>
        <span class="q-desc">{{ t("home.quickSettingsDesc") }}</span>
      </button>
    </section>

    <LogModal
      v-if="showLog && current"
      :instance-id="current.id"
      :title="t('home.log') + ' · ' + current.name"
      @close="showLog = false"
    />
  </div>
</template>

<style scoped>
.wrap {
  display: flex;
  flex-direction: column; padding: 26px 32px 60px; width: 100%; max-width: clamp(1180px, 62vw, 1800px); margin: 0 auto; box-sizing: border-box; }
.head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 22px; flex-wrap: wrap; }
.eyebrow { font-size: 10px; letter-spacing: 1.4px; color: var(--ink-3); margin-bottom: 7px; }
h1 { margin: 0; font-size: 25px; font-weight: 600; letter-spacing: -0.5px; color: var(--ink); }

.account-chip { display: flex; align-items: center; gap: 9px; padding: 7px 13px 7px 8px; border: 1px solid var(--line); border-radius: 11px; background: var(--card); cursor: pointer; transition: background 0.15s; }
.account-chip:hover { background: var(--card-hover); }
.chip-text { display: flex; flex-direction: column; align-items: flex-start; line-height: 1.25; }
.chip-name { font-size: 12px; font-weight: 500; color: var(--ink); }
.chip-kind { font-size: 9.5px; color: var(--ink-3); }

.empty { padding: 40px 28px; border: 1px dashed var(--line); border-radius: 14px; background: var(--card); margin-bottom: 22px; }
.empty-title { font-size: 16px; font-weight: 600; color: var(--ink); margin-bottom: 8px; }
.empty-desc { margin: 0 0 18px; font-size: 12px; line-height: 1.7; color: var(--ink-2); max-width: 520px; }

.launcher {
  position: relative;
  overflow: hidden;
  padding: 20px 22px;
  border: 1px solid var(--glass-border);
  border-radius: 16px;
  background: var(--glass-card);
  backdrop-filter: blur(12px) saturate(160%);
  -webkit-backdrop-filter: blur(12px) saturate(160%);
  margin-bottom: 22px;
  box-shadow: var(--glass-edge);
  transition:
    transform var(--dur-2) var(--ease-apple),
    box-shadow var(--dur-2) var(--ease-apple),
    background-color var(--dur-2) var(--ease-apple),
    border-color var(--dur-2) var(--ease-apple);
}
.launcher:hover {
  transform: translateY(-2px);
  background: var(--card-hover);
  box-shadow: var(--glass-edge-hover);
}
/* 高光斜扫（C） */
.launcher::after {
  content: "";
  position: absolute;
  inset: 0;
  pointer-events: none;
  background: linear-gradient(112deg, transparent 32%, rgba(255, 255, 255, 0.42) 48%, transparent 64%);
  background-size: 280% 100%;
  background-position: 130% 0;
  opacity: 0;
}
.launcher:hover::after {
  opacity: 1;
  animation: sheen 820ms var(--ease-out-expo);
}
.lc-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 14px; margin-bottom: 18px; flex-wrap: wrap; }
.lc-name { font-size: 27px; font-weight: 600; letter-spacing: -0.6px; color: var(--ink); line-height: 1.15; }
.lc-meta { display: flex; align-items: center; gap: 7px; flex-wrap: wrap; margin-top: 8px; font-size: 11px; color: var(--ink-3); }
.sep { opacity: 0.45; }
.tag { padding: 1.5px 7px; font-size: 9.5px; border-radius: 5px; background: var(--card-hover); color: var(--ink-3); }
.lc-path { margin-top: 7px; font-size: 10px; color: var(--ink-3); opacity: 0.6; word-break: break-all; }
.lc-time { font-size: 10.5px; color: var(--ink-3); white-space: nowrap; }

.launch {
  width: 100%;
  height: 56px;
  border: none;
  border-radius: 13px;
  background: var(--ink);
  color: var(--bg);
  font-size: 16px;
  font-weight: 600;
  letter-spacing: 0.5px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  transition:
    opacity var(--dur-2) var(--ease-apple),
    transform var(--dur-1) var(--ease-apple),
    box-shadow var(--dur-2) var(--ease-apple);
}
.launch:hover:not(:disabled) {
  opacity: 0.88;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.18);
}
.launch:active:not(:disabled) {
  transform: scale(0.975);
}
.launch:disabled { opacity: 0.5; cursor: default; }
.spinner { width: 15px; height: 15px; border: 2px solid currentColor; border-top-color: transparent; border-radius: 50%; animation: spin 0.75s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.report { margin-top: 12px; font-size: 10.5px; line-height: 1.6; color: var(--ink-2); word-break: break-all; }

.lc-foot { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-top: 16px; flex-wrap: wrap; }
.chips { display: flex; gap: 6px; flex-wrap: wrap; }
.chip { padding: 4px 11px; font-size: 11px; border: 1px solid var(--line); border-radius: 7px; background: transparent; color: var(--ink-3); cursor: pointer; transition: all 0.15s; }
.chip:hover { background: var(--card-hover); }
.chip.on { border-color: var(--ink-3); color: var(--ink); }
.foot-actions { display: flex; gap: 6px; }

.mini { padding: 4px 11px; font-size: 11px; border: 1px solid var(--line); border-radius: 7px; background: transparent; color: var(--ink-2); cursor: pointer; white-space: nowrap; }
.mini:hover { background: var(--card-hover); }

.recent { margin-bottom: 22px; }
.group-title { font-size: 11px; color: var(--ink-3); margin-bottom: 9px; }
.recent-grid { display: flex; gap: 8px; flex-wrap: wrap; }
.recent-item { display: flex; flex-direction: column; gap: 3px; align-items: flex-start; padding: 10px 14px; border: 1px solid var(--line); border-radius: 10px; background: var(--card); cursor: pointer; transition: background 0.15s; min-width: 130px; }
.recent-item:hover { background: var(--card-hover); }
.ri-name { font-size: 12.5px; font-weight: 500; color: var(--ink); }
.ri-meta { font-size: 9.5px; color: var(--ink-3); }

.quick { display: grid; grid-template-columns: repeat(3, 1fr); gap: 9px; }
.quick-item {
  display: flex;
  flex-direction: column;
  gap: 5px;
  align-items: flex-start;
  padding: 14px 16px;
  border: 1px solid var(--glass-border);
  border-radius: 12px;
  /* 跟着主题走（深色下要暗），别再硬编码白色 */
  background: var(--card);
  cursor: pointer;
  text-align: left;
  box-shadow: var(--glass-edge);
  transition:
    background-color var(--dur-2) var(--ease-apple),
    transform var(--dur-2) var(--ease-apple),
    box-shadow var(--dur-2) var(--ease-apple),
    border-color var(--dur-2) var(--ease-apple);
}
.quick-item:hover {
  background: var(--card-hover);
  transform: translateY(-2px);
  box-shadow: var(--glass-edge-hover);
  border-color: var(--glass-border);
}
.quick-item:active {
  transform: translateY(0) scale(0.99);
}
.q-title { font-size: 13px; font-weight: 600; color: var(--ink); }
.q-desc { font-size: 10.5px; color: var(--ink-3); line-height: 1.5; }

.primary { padding: 9px 18px; font-size: 12.5px; font-weight: 500; border: none; border-radius: 9px; background: var(--ink); color: var(--bg); cursor: pointer; }
.primary:hover { opacity: 0.87; }

@media (min-width: 1600px) {
  .wrap { padding: 32px 48px 72px; }
}

/* ───── PCL 手机版式启动台 ───── */

.kind-switch {
  display: flex;
  gap: 3px;
  align-self: center;
  padding: 3px;
  margin-bottom: 42px;
  border: 1px solid var(--line-2);
  border-radius: 999px;
  background: var(--card);
}

.kind {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 7px 17px;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--ink-3);
  font-size: 12.5px;
  transition:
    background var(--dur-2) var(--ease-apple),
    color var(--dur-2) var(--ease-apple);
}

.kind:hover:not(.on) { color: var(--ink); }

.kind.on {
  background: var(--ink);
  color: var(--bg);
}

.kind-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.45;
}

.kind.on .kind-dot { opacity: 1; }

.hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 13px;
  margin-bottom: 32px;
}

.hero-name {
  font-size: 22px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--ink);
}

.hero-sub {
  font-size: 12px;
  color: var(--ink-3);
  text-align: center;
}

.dot-sep { margin: 0 5px; opacity: 0.45; }

.hero-actions {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: min(520px, 100%);
  align-self: center;
  margin-bottom: 40px;
}

.big-launch {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 5px;
  padding: 17px;
  border: 1px solid var(--glass-border);
  border-radius: 16px;
  background: var(--glass-card);
  transition:
    transform var(--dur-2) var(--ease-apple),
    border-color var(--dur-2) var(--ease-apple),
    background var(--dur-2) var(--ease-apple);
}

.big-launch:hover:not(:disabled) {
  border-color: var(--ink-4);
  background: var(--card-hover);
}

.big-launch:active:not(:disabled) { transform: scale(0.988); }
.big-launch:disabled { opacity: 0.6; }

.bl-main {
  font-size: 16px;
  font-weight: 600;
  color: var(--ink);
}

.bl-sub {
  font-size: 11px;
  color: var(--ink-3);
}

.hero-row {
  display: flex;
  gap: 12px;
}

.hero-sq {
  flex: 1;
  padding: 13px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--card);
  color: var(--ink);
  font-size: 13px;
  transition:
    background var(--dur-1) var(--ease-apple),
    border-color var(--dur-1) var(--ease-apple);
}

.hero-sq:hover {
  background: var(--card-hover);
  border-color: var(--ink-4);
}

.report-line {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.log-link {
  flex: none;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--ink-3);
  font-size: 11.5px;
  text-decoration: underline;
  text-underline-offset: 2px;
}

.log-link:hover { color: var(--ink); }
</style>
