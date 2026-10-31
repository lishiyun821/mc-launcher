<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import {
  fetchVersionManifest,
  listLocalVersions,
  installVersion,
  listFabricLoaders,
  installFabric,
  launchVersion,
  getConfig,
} from "@/services/ipc";
import { useI18n } from "@/i18n";
import type { FabricLoader } from "@/services/ipc";
import type { InstallReport, LaunchReport, VersionManifest, VersionSummary } from "@/types";

const { t } = useI18n();

const manifest = ref<VersionManifest | null>(null);
const local = ref<string[]>([]);
const loading = ref(false);
const error = ref("");
const filter = ref<"release" | "all">("release");
const query = ref("");

const typeLabel = computed<Record<string, string>>(() => ({
  release: t("versions.typeRelease"),
  snapshot: t("versions.typeSnapshot"),
  old_beta: t("versions.typeOldBeta"),
  old_alpha: t("versions.typeOldAlpha"),
}));

const shown = computed<VersionSummary[]>(() => {
  const all = manifest.value?.versions ?? [];
  const base = filter.value === "release" ? all.filter((v) => v.type === "release") : all;
  const q = query.value.trim().toLowerCase();
  return q ? base.filter((v) => v.id.toLowerCase().includes(q)) : base;
});

async function load(force = false) {
  loading.value = true;
  error.value = "";
  try {
    manifest.value = await fetchVersionManifest(force);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  const cfg = await getConfig().catch(() => null);
  if (cfg?.playerName) playerName.value = cfg.playerName;
  await load();
  local.value = await listLocalVersions().catch(() => []);
});

function isLocal(id: string) {
  return local.value.includes(id);
}

function shortDate(iso: string) {
  return iso.slice(0, 10);
}

/* ───── 安装（M1） ───── */

const installingId = ref<string | null>(null);
const reports = ref<Record<string, InstallReport>>({});

async function install(v: VersionSummary) {
  if (installingId.value !== null) return;
  installingId.value = v.id;
  try {
    const r = await installVersion(v.id);
    reports.value = { ...reports.value, [v.id]: r };
    local.value = await listLocalVersions().catch(() => []);
  } catch (e) {
    console.error("[install]", e);
  } finally {
    installingId.value = null;
  }
}

/* ───── 模组加载器（Fabric）───── */

const fabricBusy = ref<string | null>(null);
const fabricReports = ref<Record<string, InstallReport>>({});

/** 选择器状态 */
const loaderOpen = ref(false);
const loaderGame = ref("");
const loaderLoading = ref(false);
const loaderError = ref("");
const loaders = ref<FabricLoader[]>([]);
const loaderPick = ref("");
const loaderQuery = ref("");

/** 列表里最新的那个 loader（用来标「最新」） */
const latestLoader = computed(() => loaders.value[0]?.version ?? "");

/** 过滤后的 loader 列表 */
const shownLoaders = computed<FabricLoader[]>(() => {
  const q = loaderQuery.value.trim().toLowerCase();
  return q ? loaders.value.filter((l) => l.version.toLowerCase().includes(q)) : loaders.value;
});

/** 拉某个游戏版本的加载器列表，默认选中最新稳定版 */
async function loadLoaders(game: string) {
  loaderLoading.value = true;
  loaderError.value = "";
  loaders.value = [];
  loaderPick.value = "";
  loaderQuery.value = "";
  try {
    const list = await listFabricLoaders(game);
    loaders.value = list;
    const preferred = list.find((l) => l.stable) ?? list[0];
    loaderPick.value = preferred?.version ?? "";
  } catch (e) {
    loaderError.value = e instanceof Error ? e.message : String(e);
  } finally {
    loaderLoading.value = false;
  }
}

function openLoaderPicker(v: VersionSummary) {
  if (installingId.value !== null || fabricBusy.value !== null) return;
  loaderGame.value = v.id;
  loaderOpen.value = true;
  void loadLoaders(v.id);
}

function closeLoaderPicker() {
  if (fabricBusy.value !== null) return;
  loaderOpen.value = false;
}

/** ESC 关闭选择器 */
function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && loaderOpen.value) closeLoaderPicker();
}

onMounted(() => window.addEventListener("keydown", onKeydown));
onBeforeUnmount(() => window.removeEventListener("keydown", onKeydown));

function retryLoaders() {
  if (loaderGame.value) void loadLoaders(loaderGame.value);
}

/** 这个 loader 装过没有 */
function loaderInstalled(ver: string) {
  return local.value.includes(`fabric-loader-${ver}-${loaderGame.value}`);
}

/** 开始安装选中的 loader */
async function confirmLoader() {
  const game = loaderGame.value;
  const pick = loaderPick.value;
  if (!game || !pick || fabricBusy.value !== null) return;

  fabricBusy.value = game;
  loaderError.value = "";
  try {
    const r = await installFabric(game, pick);
    fabricReports.value = { ...fabricReports.value, [game]: r };
    local.value = await listLocalVersions().catch(() => []);
    loaderOpen.value = false;
  } catch (e) {
    loaderError.value = e instanceof Error ? e.message : String(e);
  } finally {
    fabricBusy.value = null;
  }
}

function reportLine(r: InstallReport) {
  const mb = (r.totalBytes / 1024 / 1024).toFixed(1);
  const parts = [
    r.chain.join(" → "),
    `${r.downloaded} 下载${r.skipped ? ` / ${r.skipped} 跳过` : ""}`,
    `${r.platformLibraryCount} 库`,
    `${r.assetCount} 资源`,
    `${r.nativeExtracted} natives`,
    `${mb} MB`,
    `${(r.elapsedMs / 1000).toFixed(1)} s`,
  ];
  if (r.failed.length) parts.push(`⚠ ${r.failed.length} 个失败`);
  return parts.join("  ·  ");
}

/* ───── 启动（M3） ───── */

const playerName = ref("MyisaCN");
const launchingId = ref<string | null>(null);
const launchReports = ref<Record<string, LaunchReport>>({});
const launchError = ref("");

async function play(v: VersionSummary) {
  if (launchingId.value !== null) return;
  launchingId.value = v.id;
  launchError.value = "";
  try {
    const r = await launchVersion(v.id, playerName.value);
    launchReports.value = { ...launchReports.value, [v.id]: r };
  } catch (e) {
    launchError.value = e instanceof Error ? e.message : String(e);
  } finally {
    launchingId.value = null;
  }
}

function launchLine(r: LaunchReport) {
  return [
    `PID ${r.pid}`,
    `Java ${r.javaMajor}`,
    `${r.classpathCount} 项 classpath`,
    `${r.memoryMb} MB 堆`,
    r.account,
    r.uuid,
  ].join("  ·  ");
}
</script>

<template>
  <div class="page">
    <header>
      <div class="eyebrow">{{ t("versions.eyebrow") }}</div>
      <h1>{{ t("versions.title") }}</h1>
      <p class="sub">
        {{ t("versions.total") }}
        <span class="mono">{{ manifest?.versions.length ?? 0 }}</span>
        {{ t("versions.totalUnit") }} · {{ t("versions.latestRelease") }}
        <span class="mono">{{ manifest?.latest.release ?? "—" }}</span>
        · {{ t("versions.local") }}
        <span class="mono">{{ local.length }}</span>
        {{ t("versions.localUnit") }}
      </p>
    </header>

    <div class="toolbar">
      <div class="seg">
        <button :class="{ on: filter === 'release' }" @click="filter = 'release'">
          {{ t("versions.filterRelease") }}
        </button>
        <button :class="{ on: filter === 'all' }" @click="filter = 'all'">
          {{ t("versions.filterAll") }}
        </button>
      </div>
      <input
        v-model="query"
        class="search"
        :placeholder="t('versions.searchPlaceholder')"
        spellcheck="false"
      />
      <div class="spacer"></div>
      <button class="btn" :disabled="loading" @click="load(true)">
        {{ loading ? t("versions.refreshing") : t("versions.refresh") }}
      </button>
    </div>

    <div v-if="error" class="err mono">{{ error }}</div>
    <div v-if="launchError" class="err mono">{{ launchError }}</div>
    <div class="list">
      <div v-if="loading && !manifest" class="empty">{{ t("versions.loading") }}</div>
      <div v-else-if="!shown.length" class="empty">{{ t("versions.empty") }}</div>
      <div
        v-for="v in shown"
        :key="v.id"
        class="row"
        :class="{ busy: installingId === v.id }"
      >
        <span class="id mono">{{ v.id }}</span>
        <span class="tag" :class="v.type">{{ typeLabel[v.type] ?? v.type }}</span>
        <span v-if="v.isLatest" class="tag latest">{{ t("versions.latest") }}</span>
        <span v-if="isLocal(v.id)" class="tag local">{{ t("versions.installed") }}</span>
        <span class="spacer"></span>
        <span class="date mono">{{ shortDate(v.releaseTime) }}</span>
        <button
          v-if="isLocal(v.id)"
          class="mini play"
          :disabled="launchingId !== null || installingId !== null"
          @click="play(v)"
        >
          {{ launchingId === v.id ? t("versions.launching") : t("versions.play") }}
        </button>
        <button
          v-if="v.type === 'release'"
          class="mini fabric"
          :disabled="installingId !== null || fabricBusy !== null"
          @click="openLoaderPicker(v)"
        >
          Fabric
        </button>
        <button class="mini" :disabled="installingId !== null || fabricBusy !== null" @click="install(v)">
          {{
            installingId === v.id
              ? t("versions.installing")
              : isLocal(v.id)
                ? t("versions.reinstall")
                : t("versions.install")
          }}
        </button>
        <div v-if="reports[v.id]" class="report mono">{{ reportLine(reports[v.id]) }}</div>
        <div v-if="fabricReports[v.id]" class="report ok mono">
          ✓ Fabric · {{ reportLine(fabricReports[v.id]) }}
        </div>
        <div v-if="launchReports[v.id]" class="report ok mono">
          {{ launchLine(launchReports[v.id]) }}
        </div>
      </div>
    </div>
    <!-- ───── 加载器选择器 ───── -->
    <div v-if="loaderOpen" class="loader-mask">
      <div class="loader-panel">
        <header class="loader-head">
          <div>
            <h2>{{ t("versions.loaderTitle") }}</h2>
            <p class="loader-game mono">{{ loaderGame }}</p>
          </div>
          <button class="loader-x" :disabled="fabricBusy !== null" @click="closeLoaderPicker">✕</button>
        </header>

        <p class="loader-desc">{{ t("versions.loaderDesc") }}</p>

        <input
          v-if="loaders.length > 0"
          v-model="loaderQuery"
          class="loader-search mono"
          :placeholder="t('versions.loaderSearch')"
          spellcheck="false"
        />

        <div v-if="loaderLoading" class="loader-state">
          <span class="spinner"></span>
          <span>{{ t("versions.loaderLoading") }}</span>
        </div>

        <div v-else-if="loaderError" class="loader-state err">
          <span>{{ t("versions.loaderFailed") }}</span>
          <span class="mono msg">{{ loaderError }}</span>
          <button class="mini" @click="retryLoaders">{{ t("versions.loaderRetry") }}</button>
        </div>

        <div v-else-if="!loaders.length" class="loader-state">{{ t("versions.loaderNone") }}</div>

        <div v-else-if="!shownLoaders.length" class="loader-state">{{ t("versions.loaderNoMatch") }}</div>

        <div v-else class="loader-list">
          <button
            v-for="l in shownLoaders"
            :key="l.version"
            class="loader-item"
            :class="{ on: loaderPick === l.version, done: loaderInstalled(l.version) }"
            :disabled="fabricBusy !== null"
            @click="loaderPick = l.version"
          >
            <span class="lv mono">{{ l.version }}</span>
            <span v-if="l.stable" class="tag ok">{{ t("versions.loaderStable") }}</span>
            <span v-else-if="l.version === latestLoader" class="tag">{{ t("versions.loaderLatest") }}</span>
            <span v-else class="tag subtle">{{ t("versions.loaderBeta") }}</span>
            <span class="spacer"></span>
            <span v-if="loaderInstalled(l.version)" class="tag installed">
              ✓ {{ t("versions.loaderInstalled") }}
            </span>
            <span class="tick" :class="{ show: loaderPick === l.version }">✓</span>
          </button>
        </div>

        <footer class="loader-foot">
          <button class="ghost" :disabled="fabricBusy !== null" @click="closeLoaderPicker">
            {{ t("versions.loaderCancel") }}
          </button>
          <button class="primary" :disabled="!loaderPick || fabricBusy !== null" @click="confirmLoader">
            {{ fabricBusy !== null ? t("versions.installing") : t("versions.loaderInstall") }}
          </button>
        </footer>
      </div>
    </div>
  </div>

</template>

<style scoped>
.page {
  padding: 34px 40px 24px;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.eyebrow {
  font-size: 10px;
  letter-spacing: 0.18em;
  color: var(--ink-4);
  margin-bottom: 6px;
}

h1 {
  font-size: 27px;
  font-weight: 600;
  letter-spacing: -0.022em;
}

.sub {
  margin-top: 7px;
  font-size: 12.5px;
  color: var(--ink-3);
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 20px 0 12px;
}

.seg {
  display: flex;
  padding: 2px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.05);
}

.seg button {
  padding: 5px 11px;
  font-size: 11.5px;
  border-radius: 6px;
  color: var(--ink-3);
  transition: 0.14s;
}

.seg button.on {
  background: var(--card-hover);
  color: var(--ink);
  font-weight: 500;
  box-shadow: var(--shadow);
}

.search {
  width: 190px;
  padding: 6px 11px;
  font-size: 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--card);
  outline: none;
}

.search:focus {
  border-color: var(--ink-4);
}

.spacer {
  flex: 1;
}

.btn {
  padding: 6px 13px;
  font-size: 11.5px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--card);
  transition: 0.14s;
}

.btn:hover:not(:disabled) {
  background: var(--card-hover);
}

.btn:disabled {
  color: var(--ink-4);
  cursor: default;
}

.err {
  font-size: 11px;
  color: var(--err);
  padding: 8px 11px;
  border: 1px solid rgba(201, 48, 44, 0.25);
  border-radius: 8px;
  background: rgba(201, 48, 44, 0.06);
  margin-bottom: 10px;
}

.list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  border: 1px solid var(--line-2);
  border-radius: var(--radius);
  background: var(--card);
}

.row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 13px;
  border-bottom: 1px solid var(--line-2);
  font-size: 12px;
}

.row:last-child {
  border-bottom: none;
}

.row:hover {
  background: rgba(0, 0, 0, 0.025);
}

.id {
  font-size: 12px;
  min-width: 190px;
}

.tag {
  font-size: 9.5px;
  padding: 1.5px 6px;
  border-radius: 4px;
  background: rgba(0, 0, 0, 0.055);
  color: var(--ink-3);
}

.tag.release {
  color: var(--ok);
  background: rgba(47, 158, 95, 0.1);
}

.tag.latest {
  color: var(--ink);
  background: rgba(0, 0, 0, 0.09);
  font-weight: 600;
}

.tag.local {
  color: var(--warn);
  background: rgba(201, 131, 20, 0.11);
}

.date {
  font-size: 10.5px;
  color: var(--ink-4);
}

.empty {
  padding: 34px;
  text-align: center;
  font-size: 12px;
  color: var(--ink-3);
}
/* ───── 安装按钮与结果摘要（M1） ───── */
.row {
  flex-wrap: wrap;
  row-gap: 4px;
}

.mini.fabric {
  border-color: color-mix(in srgb, var(--ink) 28%, transparent);
  color: var(--ink);
  font-weight: 500;
}

.mini.fabric:hover:not(:disabled) {
  background: var(--ink);
  color: var(--bg);
}

.mini {
  margin-left: 8px;
  padding: 3px 10px;
  font-size: 11px;
  border-radius: 6px;
  border: 1px solid var(--line);
  background: var(--card-hover);
  color: var(--ink-2);
  white-space: nowrap;
  transition: background 0.14s, color 0.14s, border-color 0.14s;
}

.mini:hover:not(:disabled) {
  background: var(--ink);
  color: var(--bg);
  border-color: var(--ink);
}

.mini:disabled {
  opacity: 0.45;
  cursor: default;
}

.row.busy .id {
  color: var(--ink);
  font-weight: 500;
}

.report {
  flex-basis: 100%;
  margin-top: 6px;
  padding-left: 2px;
  font-size: 10px;
  line-height: 1.5;
  color: var(--ink-3);
  word-break: break-all;
}

/* 启动按钮：主色实心，和安装区分开 */
.mini.play {
  background: var(--ink);
  color: var(--bg);
  border-color: var(--ink);
}

.mini.play:hover:not(:disabled) {
  opacity: 0.82;
}

.report.ok {
  color: var(--ink-2);
}

.report.err {
  color: #d14343;
}

/* ───── 加载器选择器 ───── */

.loader-mask {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  padding: 40px;
  background: rgba(0, 0, 0, 0.38);
  animation: mask-in var(--dur-2) var(--ease-apple);
}

.loader-panel {
  width: min(560px, 100%);
  max-height: min(640px, 100%);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 22px 22px 18px;
  border-radius: 16px;
  border: 1px solid var(--line);
  background: var(--bg);
  box-shadow: var(--shadow-lg);
  animation: panel-in var(--dur-3) var(--ease-apple);
}

.loader-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.loader-head h2 {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--ink);
}

.loader-game {
  margin: 3px 0 0;
  font-size: 12px;
  color: var(--ink-3);
}

.loader-x {
  flex: none;
  width: 26px;
  height: 26px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-3);
  font-size: 13px;
  transition: background var(--dur-1) var(--ease-apple);
}

.loader-x:hover:not(:disabled) {
  background: var(--card-hover);
  color: var(--ink);
}

.loader-desc {
  margin: -4px 0 2px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--ink-3);
}

.loader-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 9px;
  padding: 26px 0;
  text-align: center;
  font-size: 13px;
  color: var(--ink-3);
}

.spinner {
  width: 13px;
  height: 13px;
  border: 1.5px solid var(--line);
  border-top-color: var(--ink-3);
  border-radius: 50%;
  animation: spin 700ms linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loader-state.err {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  color: var(--err);
}

.loader-state.err .msg {
  max-width: 100%;
  font-size: 11px;
  color: var(--ink-3);
  word-break: break-all;
}

.loader-search {
  width: 100%;
  padding: 7px 11px;
  font-size: 12px;
  border-radius: 8px;
  border: 1px solid var(--line);
  background: var(--card);
  color: var(--ink);
  outline: none;
}

.loader-search:focus {
  border-color: var(--ink-4);
}

.loader-list {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 0 -6px;
  padding: 0 6px;
  overflow-y: auto;
}

.loader-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 9px 11px;
  border-radius: 9px;
  border: 1px solid var(--line-2);
  background: var(--card);
  color: var(--ink);
  text-align: left;
  transition:
    background var(--dur-1) var(--ease-apple),
    border-color var(--dur-1) var(--ease-apple);
}

.loader-item:hover:not(:disabled) { background: var(--card-hover); }
.loader-item.on { border-color: var(--ink); background: var(--card-hover); }
.loader-item.done { opacity: 0.72; }
.loader-item .lv { font-size: 13px; }

.loader-item .spacer,
.loader-foot .spacer { flex: 1; }

.tag {
  padding: 1px 7px;
  font-size: 10px;
  letter-spacing: 0.03em;
  border-radius: 999px;
  border: 1px solid var(--line);
  color: var(--ink-3);
}

.tag.subtle {
  opacity: 0.6;
}

.tag.ok,
.tag.installed {
  border-color: color-mix(in srgb, var(--ok) 40%, transparent);
  color: var(--ok);
}

.tick {
  width: 14px;
  font-size: 12px;
  color: var(--ink);
  opacity: 0;
  transition: opacity var(--dur-1) var(--ease-apple);
}

.tick.show { opacity: 1; }

.loader-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 4px;
}

.loader-foot .ghost,
.loader-foot .primary {
  padding: 8px 18px;
  font-size: 12.5px;
  border-radius: 8px;
  border: 1px solid var(--line);
  transition:
    background var(--dur-1) var(--ease-apple),
    color var(--dur-1) var(--ease-apple);
}

.loader-foot .ghost {
  background: transparent;
  color: var(--ink-2);
}

.loader-foot .ghost:hover:not(:disabled) { background: var(--card-hover); }

.loader-foot .primary {
  background: var(--ink);
  border-color: var(--ink);
  color: var(--bg);
}

.loader-foot .primary:disabled { opacity: 0.4; }

@keyframes mask-in {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes panel-in {
  from { opacity: 0; transform: translateY(8px) scale(0.99); }
  to { opacity: 1; transform: none; }
}

@media (prefers-reduced-motion: reduce) {
  .loader-mask,
  .loader-panel { animation: none; }
}
</style>
