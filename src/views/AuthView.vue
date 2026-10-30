<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  getConfig,
  listAccounts,
  msLoginPoll,
  msLoginStart,
  refreshAccount,
  removeAccount,
  setActiveAccount,
} from "@/services/ipc";
import { useI18n } from "@/i18n";
import type { AccountInfo, DeviceCode } from "@/types";

const { t } = useI18n();

const accounts = ref<AccountInfo[]>([]);
const activeId = ref<string | null>(null);
const device = ref<DeviceCode | null>(null);
const polling = ref(false);
const error = ref("");
const copied = ref(false);
const busy = ref<string | null>(null);
const playerName = ref("MyisaCN");
const confirming = ref<string | null>(null);

let timer: number | null = null;

const activeAccount = computed(
  () => accounts.value.find((a) => a.id === activeId.value) ?? null,
);

async function load() {
  const r = await listAccounts().catch(() => null);
  if (!r) return;
  accounts.value = r.accounts;
  activeId.value = r.active;
}

onMounted(async () => {
  const cfg = await getConfig().catch(() => null);
  if (cfg?.playerName) playerName.value = cfg.playerName;
  await load();
});

onUnmounted(() => stopPolling());

/* ───────── 设备码登录 ───────── */

function stopPolling() {
  polling.value = false;
  if (timer !== null) {
    clearTimeout(timer);
    timer = null;
  }
}

async function startLogin() {
  error.value = "";
  try {
    const d = await msLoginStart();
    device.value = d;
    beginPoll(d);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function beginPoll(d: DeviceCode) {
  polling.value = true;
  const wait = Math.max(3, d.interval) * 1000;

  const tick = async () => {
    if (!polling.value || !device.value) return;

    try {
      const r = await msLoginPoll(d.deviceCode);

      if (r.state === "success") {
        stopPolling();
        device.value = null;
        await load();
        return;
      }
      if (r.state === "failed") {
        stopPolling();
        device.value = null;
        error.value = r.message;
        return;
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }

    timer = window.setTimeout(tick, wait);
  };

  timer = window.setTimeout(tick, wait);
}

function cancelLogin() {
  stopPolling();
  device.value = null;
}

async function copyCode() {
  if (!device.value) return;
  try {
    await navigator.clipboard.writeText(device.value.userCode);
    copied.value = true;
    setTimeout(() => (copied.value = false), 1600);
  } catch {
    /* 剪贴板没权限就算了，用户能自己看着输 */
  }
}

/* ───────── 账户操作 ───────── */

async function use(id: string) {
  busy.value = id;
  try {
    const r = await setActiveAccount(id);
    accounts.value = r.accounts;
    activeId.value = r.active;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = null;
  }
}

async function refresh(id: string) {
  busy.value = id;
  error.value = "";
  try {
    await refreshAccount(id);
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = null;
  }
}

async function drop(id: string) {
  confirming.value = null;
  busy.value = id;
  try {
    const r = await removeAccount(id);
    accounts.value = r.accounts;
    activeId.value = r.active;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = null;
  }
}

function fmtExpire(sec: number) {
  if (!sec) return "—";
  const d = new Date(sec * 1000);
  return d.toLocaleString();
}

/** 悬停提示：令牌具体有效到什么时候、什么时候会自动续 */
function renewTip(a: AccountInfo) {
  return t("auth.renewTip").replace("{time}", fmtExpire(a.expiresAt));
}

function initial(name: string) {
  return (name || "?").slice(0, 1).toUpperCase();
}
</script>

<template>
  <div class="wrap">
    <header class="head">
      <div>
        <h1>{{ t("auth.title") }}</h1>
        <p class="sub mono">{{ accounts.length }} {{ t("auth.count") }}</p>
      </div>
    </header>

    <div v-if="error" class="err mono">{{ error }}</div>

    <!-- 登录进行中：显示设备码 -->
    <section v-if="device" class="card login-card">
      <div class="step-title">{{ t("auth.stepTitle") }}</div>
      <ol class="steps">
        <li>{{ t("auth.step1") }}</li>
        <li>{{ t("auth.step2") }}</li>
        <li>{{ t("auth.step3") }}</li>
      </ol>

      <div class="code-line">
        <code class="code">{{ device.userCode }}</code>
        <button class="mini" @click="copyCode">
          {{ copied ? t("auth.copied") : t("auth.copy") }}
        </button>
      </div>

      <div class="link-line">
        <a :href="device.verificationUri" target="_blank" rel="noreferrer">{{
          device.verificationUri
        }}</a>
      </div>

      <div class="status-line">
        <span class="dot" :class="{ on: polling }"></span>
        <span>{{ polling ? t("auth.waiting") : t("auth.stopped") }}</span>
        <span class="dim mono">· {{ t("auth.expires") }} {{ Math.round(device.expiresIn / 60) }} min</span>
        <button class="mini" @click="cancelLogin">{{ t("auth.cancel") }}</button>
      </div>
    </section>

    <!-- 当前账户 -->
    <section v-else-if="activeAccount" class="card now">
      <div class="avatar">{{ initial(activeAccount.name) }}</div>
      <div class="now-main">
        <div class="now-name">{{ activeAccount.name }}</div>
        <div class="meta mono">{{ activeAccount.uuid }}</div>
        <div class="meta mono">
          <span class="tag" :class="activeAccount.kind === 'msa' ? 'good' : ''">
            {{ activeAccount.kind === "msa" ? t("auth.typeMsa") : t("auth.typeOffline") }}
          </span>
          <span
            v-if="activeAccount.expired"
            class="tag warn"
            :title="renewTip(activeAccount)"
          >{{ t("auth.expiredAuto") }}</span>
          <span
            v-else-if="activeAccount.kind === 'msa'"
            class="tag"
            :title="renewTip(activeAccount)"
          >{{ t("auth.autoRenew") }}</span>
        </div>
      </div>
      <div class="now-actions">
        <button
          v-if="activeAccount.kind === 'msa'"
          class="mini"
          :disabled="busy === activeAccount.id"
          @click="refresh(activeAccount.id)"
        >
          {{ busy === activeAccount.id ? t("auth.refreshing") : t("auth.refresh") }}
        </button>
        <button class="mini" @click="startLogin">{{ t("auth.addAnother") }}</button>
      </div>
    </section>

    <!-- 没账户 -->
    <section v-else class="card empty-state">
      <div class="empty-title">{{ t("auth.notLoggedIn") }}</div>
      <p class="empty-desc">{{ t("auth.loginDesc") }}</p>
      <div class="empty-actions">
        <button class="mini play" @click="startLogin">{{ t("auth.loginMs") }}</button>
      </div>
      <p class="hint mono">
        {{ t("auth.offlineHint").replace("{name}", playerName) }}
      </p>
    </section>

    <!-- 账户列表 -->
    <section v-if="accounts.length" class="list">
      <div class="group-title">{{ t("auth.allAccounts") }}</div>
      <div v-for="a in accounts" :key="a.id" class="row" :class="{ on: a.id === activeId }">
        <div class="avatar small">{{ initial(a.name) }}</div>
        <div class="row-main">
          <div class="row-name">
            {{ a.name }}
            <span v-if="a.id === activeId" class="tag good">{{ t("auth.current") }}</span>
            <span v-if="a.expired" class="tag warn">{{ t("auth.expired") }}</span>
          </div>
          <div class="meta mono">{{ a.uuid }} · {{ a.kind === "msa" ? t("auth.typeMsa") : t("auth.typeOffline") }}</div>
        </div>
        <div class="row-actions">
          <button v-if="a.id !== activeId" class="mini" :disabled="busy === a.id" @click="use(a.id)">
            {{ t("auth.use") }}
          </button>
          <button
            v-if="a.kind === 'msa'"
            class="mini"
            :disabled="busy === a.id"
            @click="refresh(a.id)"
          >
            {{ t("auth.refresh") }}
          </button>
          <button class="mini danger" @click="confirming = a.id">{{ t("auth.remove") }}</button>
        </div>
        <div v-if="confirming === a.id" class="confirm">
          <span>{{ t("auth.confirmRemove") }}</span>
          <button class="mini danger" @click="drop(a.id)">{{ t("auth.confirmYes") }}</button>
          <button class="mini" @click="confirming = null">{{ t("auth.cancel") }}</button>
        </div>
      </div>
    </section>
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

.err {
  margin-bottom: 12px;
  font-size: 11px;
  color: #d14343;
  word-break: break-all;
}

.card {
  padding: 16px 18px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--card);
}

/* ───── 登录卡 ───── */

.login-card {
  margin-bottom: 14px;
}

.step-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--ink);
  margin-bottom: 10px;
}

.steps {
  margin: 0 0 14px;
  padding-left: 18px;
  font-size: 11.5px;
  line-height: 1.9;
  color: var(--ink-2);
}

.code-line {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.code {
  font-family: var(--mono, ui-monospace, "SF Mono", Menlo, monospace);
  font-size: 26px;
  font-weight: 600;
  letter-spacing: 3px;
  color: var(--ink);
  padding: 8px 16px;
  border-radius: 9px;
  background: var(--card-hover);
  user-select: all;
}

.link-line a {
  font-size: 12px;
  color: var(--ink-2);
  text-decoration: underline;
  text-underline-offset: 2px;
}

.status-line {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 14px;
  font-size: 11.5px;
  color: var(--ink-3);
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--ink-3);
  opacity: 0.4;
}

.dot.on {
  background: #35a55b;
  opacity: 1;
  animation: pulse 1.4s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.dim {
  opacity: 0.65;
}

/* ───── 当前账户 ───── */

.now {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 22px;
  flex-wrap: wrap;
}

.avatar {
  width: 52px;
  height: 52px;
  border-radius: 14px;
  display: grid;
  place-items: center;
  font-size: 21px;
  font-weight: 600;
  color: var(--bg);
  background: var(--ink);
  flex-shrink: 0;
}

.avatar.small {
  width: 32px;
  height: 32px;
  border-radius: 9px;
  font-size: 14px;
}

.now-main {
  flex: 1;
  min-width: 200px;
}

.now-name {
  font-size: 17px;
  font-weight: 600;
  color: var(--ink);
}

.meta {
  margin-top: 3px;
  font-size: 10.5px;
  color: var(--ink-3);
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.now-actions,
.row-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

/* ───── 空状态 ───── */

.empty-state {
  margin-bottom: 22px;
}

.empty-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--ink);
  margin-bottom: 6px;
}

.empty-desc {
  margin: 0 0 14px;
  font-size: 12px;
  line-height: 1.7;
  color: var(--ink-2);
  max-width: 560px;
}

.empty-actions {
  margin-bottom: 12px;
}

.hint {
  margin: 0;
  font-size: 10.5px;
  line-height: 1.7;
  color: var(--ink-3);
  opacity: 0.85;
}

/* ───── 账户列表 ───── */

.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.group-title {
  font-size: 11px;
  color: var(--ink-3);
  margin-bottom: 2px;
}

.row {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 10px 13px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--card);
  flex-wrap: wrap;
}

.row.on {
  border-color: var(--ink-3);
}

.row-main {
  flex: 1;
  min-width: 180px;
}

.row-name {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--ink);
  display: flex;
  align-items: center;
  gap: 6px;
}

.tag {
  padding: 1px 6px;
  font-size: 9.5px;
  border-radius: 5px;
  background: var(--card-hover);
  color: var(--ink-3);
  font-weight: 400;
}

.tag.good {
  background: rgba(53, 165, 91, 0.12);
  color: #35a55b;
}

.tag.warn {
  background: rgba(209, 67, 67, 0.1);
  color: #d14343;
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

.confirm {
  flex-basis: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
  padding: 8px 10px;
  border-radius: 8px;
  background: rgba(209, 67, 67, 0.06);
  font-size: 11.5px;
  color: var(--ink-2);
}

@media (min-width: 1600px) {
  .wrap { padding: 32px 48px 72px; }
}
</style>
