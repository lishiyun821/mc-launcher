<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { fetchVersionManifest, listLocalVersions, installVersion, launchVersion, getConfig } from "@/services/ipc";
import { useI18n } from "@/i18n";
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
        <button class="mini" :disabled="installingId !== null" @click="install(v)">
          {{
            installingId === v.id
              ? t("versions.installing")
              : isLocal(v.id)
                ? t("versions.reinstall")
                : t("versions.install")
          }}
        </button>
        <div v-if="reports[v.id]" class="report mono">{{ reportLine(reports[v.id]) }}</div>
        <div v-if="launchReports[v.id]" class="report ok mono">
          {{ launchLine(launchReports[v.id]) }}
        </div>
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
  backdrop-filter: blur(18px) saturate(180%);
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
</style>
