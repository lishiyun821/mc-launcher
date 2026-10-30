<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  deleteInstance,
  duplicateInstance,
  getConfig,
  getInstance,
  launchVersion,
  listInstances,
  openGameFolder,
  openInstanceFolder,
  updateInstance,
  verifyInstance,
} from "@/services/ipc";
import { useI18n } from "@/i18n";
import type { InstanceConfig, InstanceInfo, VerifyReport } from "@/types";
import LogModal from "@/components/LogModal.vue";

const { t } = useI18n();

const items = ref<InstanceInfo[]>([]);
const loading = ref(true);
const error = ref("");
const playerName = ref("MyisaCN");

const launchingId = ref<string | null>(null);
const reports = ref<Record<string, string>>({});

/** 正在编辑设置的实例 */
const editing = ref<string | null>(null);
const draft = ref<InstanceConfig | null>(null);
const saving = ref(false);

/** 删除二次确认 */
const confirming = ref<string | null>(null);

/** 日志弹窗 */
const logTarget = ref<InstanceInfo | null>(null);

/** 校验状态 */
const verifying = ref<string | null>(null);
const verifyResults = ref<Record<string, VerifyReport>>({});

const totalSize = computed(() => items.value.reduce((s, i) => s + i.sizeBytes, 0));

async function load() {
  loading.value = true;
  error.value = "";
  try {
    items.value = await listInstances();
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
});

function fmtSize(bytes: number) {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function fmtAgo(sec: number | null) {
  if (!sec) return "还没玩过";
  const diff = Date.now() / 1000 - sec;
  if (diff < 60) return "刚刚玩过";
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
  if (diff < 86400 * 30) return `${Math.floor(diff / 86400)} 天前`;
  return new Date(sec * 1000).toLocaleDateString();
}

function kindLabel(kind: string | null) {
  if (kind === "snapshot") return "快照";
  if (kind === "old_beta") return "Beta";
  if (kind === "old_alpha") return "Alpha";
  return "正式版";
}

/* ───────── 启动 ───────── */

async function play(it: InstanceInfo) {
  if (launchingId.value) return;
  launchingId.value = it.id;
  reports.value = { ...reports.value, [it.id]: "正在准备…" };
  try {
    const r = await launchVersion(it.id, playerName.value);
    reports.value = {
      ...reports.value,
      [it.id]: `已启动 · PID ${r.pid} · Java ${r.javaMajor} · ${r.classpathCount} 项 classpath · ${r.memoryMb} MB 堆`,
    };
    // 刷新「上次游玩」
    setTimeout(load, 800);
  } catch (e) {
    reports.value = {
      ...reports.value,
      [it.id]: `启动失败: ${e instanceof Error ? e.message : String(e)}`,
    };
  } finally {
    launchingId.value = null;
  }
}

/* ───────── 设置 ───────── */

async function openEdit(it: InstanceInfo) {
  if (editing.value === it.id) {
    editing.value = null;
    draft.value = null;
    return;
  }
  const cfg = await getInstance(it.id).catch(() => null);
  if (!cfg) return;
  editing.value = it.id;
  draft.value = { ...cfg, extraJvm: [...cfg.extraJvm] };
}

async function saveEdit(it: InstanceInfo) {
  if (!draft.value || saving.value) return;
  saving.value = true;
  try {
    await updateInstance(it.id, draft.value);
    editing.value = null;
    draft.value = null;
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

/* ───────── 删除 / 复制 / 打开目录 ───────── */

function askDelete(id: string) {
  confirming.value = id;
}

async function doDelete(it: InstanceInfo) {
  confirming.value = null;
  try {
    await deleteInstance(it.id);
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

async function doDuplicate(it: InstanceInfo) {
  const name = `${it.id}-copy`;
  try {
    await duplicateInstance(it.id, name);
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

async function openFolder(it: InstanceInfo) {
  await openInstanceFolder(it.id).catch((e) => {
    error.value = e instanceof Error ? e.message : String(e);
  });
}

function openLog(it: InstanceInfo) {
  logTarget.value = it;
}

async function doVerify(it: InstanceInfo) {
  if (verifying.value) return;
  verifying.value = it.id;
  try {
    const r = await verifyInstance(it.id);
    verifyResults.value = { ...verifyResults.value, [it.id]: r };
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    verifying.value = null;
  }
}

function verifyLine(r: VerifyReport) {
  if (r.ok) {
    return `${t("instances.verifyOk")} · ${r.total} 项 classpath · ${r.nativeFiles} natives · ${r.checkedMs} ms`;
  }
  return `${t("instances.verifyBad")} · 缺 ${r.missing.length}/${r.total} 项（例: ${r.missing[0] ?? "-"}）`;
}
</script>

<template>
  <div class="wrap">
    <header class="head">
      <div>
        <h1>{{ t("instances.title") }}</h1>
        <p class="sub mono">
          {{ items.length }} {{ t("instances.count") }} · {{ fmtSize(totalSize) }}
        </p>
      </div>
      <button class="ghost" @click="openGameFolder()">{{ t("instances.openGame") }}</button>
    </header>

    <div v-if="error" class="err mono">{{ error }}</div>

    <div v-if="loading && !items.length" class="empty">{{ t("instances.loading") }}</div>
    <div v-else-if="!items.length" class="empty">
      {{ t("instances.empty") }}
    </div>

    <div class="list">
      <div v-for="it in items" :key="it.id" class="card" :class="{ busy: launchingId === it.id }">
        <div class="row">
          <div class="main">
            <div class="title-line">
              <span class="name">{{ it.name }}</span>
              <span class="tag">{{ kindLabel(it.kind) }}</span>
              <span v-if="it.id !== it.name" class="tag dim mono">{{ it.id }}</span>
              <span v-if="!it.hasJar" class="tag warn">{{ t("instances.noJar") }}</span>
            </div>
            <div class="meta mono">
              {{ it.libraryCount }} {{ t("instances.libs") }} · {{ fmtSize(it.sizeBytes) }} ·
              {{ it.memoryMb ? `${it.memoryMb} MB` : t("instances.defaultMemory") }} ·
              Java {{ it.javaMajor ?? "?" }} · {{ fmtAgo(it.lastPlayed) }}
            </div>
            <div class="meta mono dim-path">{{ it.gameDir }}</div>
          </div>

          <div class="actions">
            <button
              class="mini play"
              :disabled="launchingId !== null"
              @click="play(it)"
            >
              {{ launchingId === it.id ? t("instances.launching") : t("instances.play") }}
            </button>
            <button class="mini" @click="openEdit(it)">
              {{ editing === it.id ? t("instances.close") : t("instances.settings") }}
            </button>
            <button class="mini" @click="openFolder(it)">{{ t("instances.folder") }}</button>
            <button class="mini" @click="openLog(it)">{{ t("instances.log") }}</button>
            <button class="mini" :disabled="verifying === it.id" @click="doVerify(it)">
              {{ verifying === it.id ? t("instances.verifying") : t("instances.verify") }}
            </button>
            <button class="mini" @click="doDuplicate(it)">{{ t("instances.duplicate") }}</button>
            <button class="mini danger" @click="askDelete(it.id)">{{ t("instances.remove") }}</button>
          </div>
        </div>

        <!-- 启动结果 -->
        <div
          v-if="reports[it.id]"
          class="report mono"
          :class="{ bad: reports[it.id].startsWith('启动失败') }"
        >
          {{ reports[it.id] }}
        </div>

        <div
          v-if="verifyResults[it.id]"
          class="report mono"
          :class="{ bad: !verifyResults[it.id].ok }"
        >
          {{ verifyLine(verifyResults[it.id]) }}
        </div>

        <!-- 删除确认 -->
        <div v-if="confirming === it.id" class="confirm">
          <span>{{ t("instances.confirmRemove") }}</span>
          <button class="mini danger" @click="doDelete(it)">{{ t("instances.confirmYes") }}</button>
          <button class="mini" @click="confirming = null">{{ t("instances.cancel") }}</button>
        </div>

        <!-- 设置面板 -->
        <div v-if="editing === it.id && draft" class="editor">
          <div class="field">
            <label>{{ t("instances.name") }}</label>
            <input v-model="draft.name" class="input" maxlength="32" />
          </div>
          <div class="field">
            <label>{{ t("instances.memory") }}</label>
            <input
              v-model.number="draft.memoryMb"
              class="input mono"
              type="number"
              min="512"
              max="32768"
              step="512"
              :placeholder="t('instances.memoryHint')"
            />
          </div>
          <div class="field">
            <label>{{ t("instances.java") }}</label>
            <input v-model="draft.javaPath" class="input mono wide" :placeholder="t('instances.javaHint')" />
          </div>
          <div class="field">
            <label>{{ t("instances.window") }}</label>
            <div class="inline">
              <input v-model.number="draft.width" class="input mono narrow" type="number" placeholder="854" />
              <span class="x">×</span>
              <input v-model.number="draft.height" class="input mono narrow" type="number" placeholder="480" />
            </div>
          </div>
          <div class="field">
            <label>{{ t("instances.gameDir") }}</label>
            <input v-model="draft.gameDir" class="input mono wide" :placeholder="t('instances.gameDirHint')" />
          </div>
          <div class="field full">
            <label>{{ t("instances.extraJvm") }}</label>
            <input
              class="input mono wide"
              :value="draft.extraJvm.join(' ')"
              placeholder="-XX:+UseG1GC"
              @input="draft.extraJvm = ($event.target as HTMLInputElement).value.split(/\s+/).filter(Boolean)"
            />
          </div>
          <div class="field full actions-row">
            <button class="mini play" :disabled="saving" @click="saveEdit(it)">
              {{ saving ? t("instances.saving") : t("instances.save") }}
            </button>
            <button class="mini" @click="editing = null; draft = null">
              {{ t("instances.cancel") }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <LogModal
      v-if="logTarget"
      :instance-id="logTarget.id"
      :title="t('instances.log') + ' · ' + logTarget.name"
      @close="logTarget = null"
    />
  </div>
</template>

<style scoped>
.wrap {
  padding: 26px 32px 60px;
  width: 100%;
  max-width: clamp(1180px, 62vw, 1800px);
  margin: 0 auto;
  box-sizing: border-box;
}

.head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 18px;
}

h1 {
  margin: 0;
  font-size: 21px;
  font-weight: 600;
  letter-spacing: -0.2px;
  color: var(--ink);
}

.sub {
  margin: 4px 0 0;
  font-size: 11px;
  color: var(--ink-3);
}

.ghost {
  padding: 6px 13px;
  font-size: 11.5px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: transparent;
  color: var(--ink-2);
  cursor: pointer;
  transition: background 0.15s;
}

.ghost:hover {
  background: var(--card-hover);
}

.empty {
  padding: 40px 0;
  text-align: center;
  font-size: 12px;
  color: var(--ink-3);
}

.err {
  margin-bottom: 12px;
  font-size: 11px;
  color: #d14343;
  word-break: break-all;
}

.list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.card {
  padding: 14px 16px;
  border: 1px solid var(--glass-border);
  border-radius: 12px;
  background: var(--card);
  box-shadow: var(--glass-edge);
  transition: border-color 0.2s, box-shadow var(--dur-2) var(--ease-apple);
}

.card.busy {
  border-color: var(--ink-3);
}

.row {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  flex-wrap: wrap;
}

.main {
  flex: 1;
  min-width: 260px;
}

.title-line {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.name {
  font-size: 14.5px;
  font-weight: 600;
  color: var(--ink);
}

.tag {
  padding: 1.5px 7px;
  font-size: 9.5px;
  border-radius: 5px;
  background: var(--card-hover);
  color: var(--ink-3);
}

.tag.dim {
  color: var(--ink-3);
  opacity: 0.75;
}

.tag.warn {
  background: rgba(209, 67, 67, 0.1);
  color: #d14343;
}

.meta {
  margin-top: 5px;
  font-size: 10.5px;
  color: var(--ink-3);
}

.dim-path {
  opacity: 0.6;
  word-break: break-all;
}

.actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.mini {
  padding: 4px 11px;
  font-size: 11px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: transparent;
  color: var(--ink-2);
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.mini:hover:not(:disabled) {
  background: var(--card-hover);
}

.mini:disabled {
  opacity: 0.45;
  cursor: default;
}

.mini.play {
  background: var(--ink);
  color: var(--bg);
  border-color: var(--ink);
}

.mini.play:hover:not(:disabled) {
  opacity: 0.82;
}

.mini.danger {
  color: #d14343;
  border-color: rgba(209, 67, 67, 0.35);
}

.mini.danger:hover:not(:disabled) {
  background: rgba(209, 67, 67, 0.08);
}

.report {
  margin-top: 10px;
  padding-top: 9px;
  border-top: 1px solid var(--line);
  font-size: 10px;
  line-height: 1.6;
  color: var(--ink-2);
  word-break: break-all;
}

.report.bad {
  color: #d14343;
}

.confirm {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
  padding: 9px 11px;
  border-radius: 8px;
  background: rgba(209, 67, 67, 0.06);
  font-size: 11.5px;
  color: var(--ink-2);
}

.editor {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 11px 14px;
  margin-top: 12px;
  padding-top: 13px;
  border-top: 1px solid var(--line);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.field.full {
  grid-column: 1 / -1;
}

.field label {
  font-size: 10.5px;
  color: var(--ink-3);
}

.input {
  padding: 6px 9px;
  font-size: 11.5px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--bg);
  color: var(--ink);
  outline: none;
}

.input:focus {
  border-color: var(--ink-3);
}

.input.wide {
  width: 100%;
}

.input.narrow {
  width: 74px;
}

.inline {
  display: flex;
  align-items: center;
  gap: 6px;
}

.x {
  font-size: 11px;
  color: var(--ink-3);
}

.actions-row {
  flex-direction: row;
  gap: 8px;
}

@media (min-width: 1600px) {
  .wrap { padding: 32px 48px 72px; }
}
</style>
