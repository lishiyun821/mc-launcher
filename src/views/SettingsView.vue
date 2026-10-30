<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getConfig, setConfig, scanJava, installJava, checkUpdate, type JavaInfo, type UpdateInfo } from "@/services/ipc";
import { useI18n, languages } from "@/i18n";
import { applyTheme } from "@/services/theme";
import type { AppConfig, Lang } from "@/types";

const { t, setLang } = useI18n();

const sources = computed(() => [
  { value: "official", label: t("settings.official") },
  { value: "mirror", label: t("settings.mirror") },
]);

const config = ref<AppConfig | null>(null);
const javas = ref<JavaInfo[]>([]);
const saved = ref(false);
const loadingJava = ref(true);

onMounted(async () => {
  config.value = await getConfig().catch(() => null);
  javas.value = await scanJava().catch(() => []);
  loadingJava.value = false;

  // 进设置页就静默查一次更新，用户不点也知道自己是不是最新。
  // 不置 checking，避免按钮闪一下"检查中"；失败也不打扰。
  checkUpdate()
    .then((r) => {
      upd.value = r;
    })
    .catch(() => {});
});

/** 语言：选了立刻生效（不点保存也生效），并落盘 */
function onLangChange(value: string) {
  const lang = value as Lang;
  setLang(lang);
  if (config.value) {
    config.value.language = lang;
    setConfig(config.value).catch(() => {});
  }
}

/** 下载源：同样立刻持久化 */
function onSourceChange(value: string) {
  if (!config.value) return;
  config.value.downloadSource = value as AppConfig["downloadSource"];
  setConfig(config.value).catch(() => {});
}

/** 玩家名：改完立刻存（启动时直接读这份） */
function onPlayerNameChange(value: string) {
  if (!config.value) return;
  config.value.playerName = value.trim();
  setConfig(config.value).catch(() => {});
}

/** 主题：立刻应用 + 落盘 */
function onThemeChange(value: string) {
  applyTheme(value);
  if (!config.value) return;
  config.value.theme = value;
  setConfig(config.value).catch(() => {});
}

/** 下载限速（KB/s，0 = 不限） */
function onLimitChange(value: string) {
  if (!config.value) return;
  const n = Number(value);
  config.value.downloadLimitKbps = Number.isFinite(n) && n > 0 ? Math.round(n) : 0;
  setConfig(config.value).catch(() => {});
}

const themeOptions = computed(() => [
  { value: "system", label: t("settings.themeSystem") },
  { value: "light", label: t("settings.themeLight") },
  { value: "dark", label: t("settings.themeDark") },
]);

/* ───────── 更新检查 ───────── */

const checking = ref(false);
const upd = ref<UpdateInfo | null>(null);

async function doCheckUpdate() {
  if (checking.value) return;
  checking.value = true;
  try {
    upd.value = await checkUpdate();
  } catch (e) {
    upd.value = {
      current: "?",
      latest: null,
      hasUpdate: false,
      url: "",
      notes: null,
      publishedAt: null,
      error: e instanceof Error ? e.message : String(e),
    };
  } finally {
    checking.value = false;
  }
}

/* ───────── Java 自动下载 ───────── */

const javaBusy = ref<number | null>(null);
const javaMsg = ref("");

async function downloadJava(major: number) {
  if (javaBusy.value !== null) return;
  javaBusy.value = major;
  javaMsg.value = `${t("settings.javaDownloading")} Java ${major}…`;
  try {
    const path = await installJava(major);
    javaMsg.value = `Java ${major} ✓ ${path}`;
    // 重新扫一遍，把它显示进列表
    javas.value = await scanJava().catch(() => javas.value);
  } catch (e) {
    javaMsg.value = `${t("settings.javaFailed")}${e instanceof Error ? e.message : String(e)}`;
  } finally {
    javaBusy.value = null;
  }
}

async function save() {
  if (!config.value) return;
  await setConfig(config.value);
  saved.value = true;
  setTimeout(() => (saved.value = false), 1600);
}
</script>

<template>
  <div class="page">
    <header>
      <div class="eyebrow">{{ t("settings.eyebrow") }}</div>
      <h1>{{ t("settings.title") }}</h1>
      <p class="sub">{{ t("settings.sub") }}</p>
    </header>

    <section v-if="config" class="group">
      <div class="group-title">{{ t("settings.groupSource") }}</div>
      <div class="row">
        <div class="row-main">
          <div class="label">{{ t("settings.preferSource") }}</div>
          <div class="desc">{{ t("settings.preferDesc") }}</div>
        </div>
        <div class="seg">
          <button
            v-for="s in sources"
            :key="s.value"
            :class="{ on: config.downloadSource === s.value }"
            @click="onSourceChange(s.value)"
          >
            {{ s.label }}
          </button>
        </div>
      </div>
      <div class="row">
        <div class="row-main">
          <div class="label">{{ t("settings.language") }}</div>
          <div class="desc">{{ t("settings.languageDesc") }}</div>
        </div>
        <div class="seg">
          <button
            v-for="l in languages"
            :key="l.value"
            :class="{ on: config.language === l.value }"
            @click="onLangChange(l.value)"
          >
            {{ l.label }}
          </button>
        </div>
      </div>
      <div class="row">
        <div class="row-main">
          <div class="label">{{ t("settings.theme") }}</div>
          <div class="desc">{{ t("settings.themeDesc") }}</div>
        </div>
        <div class="seg">
          <button
            v-for="opt in themeOptions"
            :key="opt.value"
            :class="{ on: (config?.theme ?? 'system') === opt.value }"
            @click="onThemeChange(opt.value)"
          >
            {{ opt.label }}
          </button>
        </div>
      </div>
      <div class="row">
        <div class="row-main">
          <div class="label">{{ t("settings.downloadLimit") }}</div>
          <div class="desc">{{ t("settings.downloadLimitDesc") }}</div>
        </div>
        <input
          class="input mono"
          type="number"
          min="0"
          step="256"
          :value="config?.downloadLimitKbps ?? 0"
          @change="onLimitChange(($event.target as HTMLInputElement).value)"
        />
      </div>
      <div class="row">
        <div class="row-main">
          <div class="label">{{ t("versions.playerName") }}</div>
          <div class="desc">离线账户名字（3-16 位，字母数字下划线）</div>
        </div>
        <input
          v-model="config.playerName"
          class="input mono"
          maxlength="16"
          spellcheck="false"
          @change="onPlayerNameChange(($event.target as HTMLInputElement).value)"
        />
      </div>
    </section>

    <section class="group">
      <div class="group-title">{{ t("settings.groupJava") }}</div>
      <div v-if="loadingJava" class="row">
        <div class="row-main">
          <div class="label">…</div>
        </div>
      </div>
      <div v-else-if="!javas.length" class="row">
        <div class="row-main">
          <div class="label">{{ t("settings.noJava") }}</div>
          <div class="desc">{{ t("settings.noJavaDesc") }}</div>
        </div>
      </div>
      <div v-for="j in javas" :key="j.path" class="row">
        <div class="row-main">
          <div class="label mono">{{ j.version }}</div>
          <div class="desc mono">{{ j.path }}</div>
        </div>
        <span class="badge">{{ j.major }}</span>
        <span v-if="j.is64Bit" class="badge">{{ t("settings.bit64") }}</span>
      </div>

      <div class="row">
        <div class="row-main">
          <div class="label">{{ t("settings.javaAuto") }}</div>
          <div class="desc">{{ t("settings.javaAutoDesc") }}</div>
        </div>
        <div class="seg">
          <button
            v-for="m in [8, 17, 21]"
            :key="m"
            :disabled="javaBusy !== null"
            @click="downloadJava(m)"
          >
            {{ javaBusy === m ? "…" : "Java " + m }}
          </button>
        </div>
      </div>
      <div v-if="javaMsg" class="row">
        <div class="row-main">
          <div class="desc mono">{{ javaMsg }}</div>
        </div>
      </div>
    </section>

    <div class="actions">
      <button class="btn primary" @click="save">{{ t("settings.save") }}</button>
      <span v-if="saved" class="ok">{{ t("settings.saved") }}</span>
    </div>

    <section class="group">
      <div class="group-title">{{ t("settings.about") }}</div>
      <div class="row about-row">
        <div class="about-icon" aria-hidden="true">M</div>
        <div class="row-main">
          <div class="label">MC Launcher</div>
          <div class="about-meta">
            <span class="ver mono">v{{ upd?.current || "0.1.0" }}</span>
            <span class="dot">·</span>
            <span>MIT</span>
            <span class="dot">·</span>
            <a
              class="link"
              href="https://github.com/lishiyun821/mc-launcher"
              target="_blank"
              rel="noopener"
            >GitHub</a>
          </div>
          <div class="about-status">
            <span v-if="checking" class="st">{{ t("settings.checking") }}</span>
            <a
              v-else-if="upd?.hasUpdate"
              class="st new"
              :href="upd.url"
              target="_blank"
              rel="noopener"
            >{{ t("settings.newVersion") }} {{ upd.latest }} → {{ t("settings.goDownload") }}</a>
            <span v-else-if="upd?.error" class="st warn">{{ upd.error }}</span>
            <span v-else-if="upd" class="st ok">✓ {{ t("settings.upToDate") }}</span>
          </div>
        </div>
        <button class="btn" :disabled="checking" @click="doCheckUpdate">
          {{ checking ? t("settings.checking") : t("settings.checkUpdate") }}
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.page {
  padding: 34px 40px;
  overflow: auto;
  height: 100%;
  width: 100%;
  max-width: clamp(1180px, 64vw, 1800px);
  margin: 0 auto;
  box-sizing: border-box;
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

.group {
  margin-top: 26px;
  border: 1px solid var(--glass-border);
  border-radius: var(--radius);
  background: var(--card);
  box-shadow: var(--glass-edge);
  overflow: hidden;
}

.group-title {
  padding: 9px 15px;
  font-size: 10px;
  letter-spacing: 0.14em;
  color: var(--ink-3);
  border-bottom: 1px solid var(--line-2);
  text-transform: uppercase;
}

.row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px 15px;
  border-bottom: 1px solid var(--line-2);
}

.row:last-child {
  border-bottom: none;
}

.row-main {
  flex: 1;
  min-width: 0;
}

.label {
  font-size: 12.5px;
  font-weight: 500;
}

.desc {
  font-size: 11px;
  color: var(--ink-3);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.select {
  padding: 5px 9px;
  font-size: 11.5px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--card-hover);
  outline: none;
}

/* 分段控件（替代原生 select，点击即生效） */
.seg {
  display: flex;
  gap: 2px;
  padding: 2px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.055);
}

.seg button {
  padding: 5px 11px;
  font-size: 11.5px;
  border-radius: 6px;
  color: var(--ink-3);
  transition: background 0.14s, color 0.14s;
  white-space: nowrap;
}

.seg button:hover {
  color: var(--ink-2);
}

.seg button.on {
  background: var(--card-hover);
  color: var(--ink);
  font-weight: 500;
  box-shadow: var(--shadow);
}

/* 文本输入框（玩家名） */
.input {
  width: 150px;
  padding: 5px 9px;
  font-size: 11.5px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--card-hover);
  color: var(--ink);
  outline: none;
  text-align: right;
}

.input:focus {
  border-color: var(--ink-3);
}

.badge {
  font-size: 10px;
  padding: 1.5px 7px;
  border-radius: 5px;
  background: rgba(0, 0, 0, 0.055);
  color: var(--ink-3);
}

.actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 22px;
}

.btn {
  padding: 7px 16px;
  font-size: 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--card);
}

.btn.primary {
  background: var(--ink);
  color: var(--bg);
  border-color: var(--ink);
  font-weight: 500;
}

.btn.primary:hover {
  opacity: 0.86;
}

.ok {
  font-size: 11.5px;
  color: var(--ok);
}

.link {
  color: var(--ink);
  text-decoration: underline;
  text-underline-offset: 2px;
}

.warn {
  color: #b4763a;
}

/* ── 关于 ── */
.about-row {
  padding: 14px 15px;
  align-items: center;
}

.about-icon {
  width: 38px;
  height: 38px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border-radius: 9px;
  background: var(--ink);
  color: var(--bg);
  font-size: 16px;
  font-weight: 600;
  letter-spacing: -0.5px;
}

.about-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 4px;
  font-size: 11px;
  color: var(--ink-3);
}

.about-meta .ver {
  padding: 1px 6px;
  border-radius: 5px;
  background: rgba(0, 0, 0, 0.055);
  font-size: 10px;
  color: var(--ink-2);
}

.about-meta .dot {
  opacity: 0.4;
}

.about-meta .link {
  color: var(--ink-2);
  text-decoration: none;
  transition: color var(--dur-2) var(--ease-apple);
}

.about-meta .link:hover {
  color: var(--ink);
  text-decoration: underline;
}

.about-status {
  margin-top: 6px;
  font-size: 11px;
  line-height: 1.4;
}

.about-status .st {
  color: var(--ink-3);
  text-decoration: none;
}

.about-status .st.ok {
  color: var(--ok);
}

.about-status .st.new {
  color: var(--ink);
  font-weight: 500;
}

.about-status .st.new:hover {
  text-decoration: underline;
}

.about-status .st.warn {
  color: var(--warn);
}

/* 深色下版本号的底要提亮 */
:root[data-theme="dark"] .about-meta .ver {
  background: rgba(255, 255, 255, 0.08);
}

@media (prefers-color-scheme: dark) {
  :root:not([data-theme]) .about-meta .ver {
    background: rgba(255, 255, 255, 0.08);
  }
}
</style>
