<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { fetchVersionManifest, listLocalVersions } from "@/services/ipc";
import { useI18n } from "@/i18n";
import type { VersionManifest, VersionSummary } from "@/types";

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
  await load();
  local.value = await listLocalVersions().catch(() => []);
});

function isLocal(id: string) {
  return local.value.includes(id);
}

function shortDate(iso: string) {
  return iso.slice(0, 10);
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

    <div class="list">
      <div v-if="loading && !manifest" class="empty">{{ t("versions.loading") }}</div>
      <div v-else-if="!shown.length" class="empty">{{ t("versions.empty") }}</div>
      <div v-for="v in shown" :key="v.id" class="row">
        <span class="id mono">{{ v.id }}</span>
        <span class="tag" :class="v.type">{{ typeLabel[v.type] ?? v.type }}</span>
        <span v-if="v.isLatest" class="tag latest">{{ t("versions.latest") }}</span>
        <span v-if="isLocal(v.id)" class="tag local">{{ t("versions.installed") }}</span>
        <span class="spacer"></span>
        <span class="date mono">{{ shortDate(v.releaseTime) }}</span>
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
</style>
