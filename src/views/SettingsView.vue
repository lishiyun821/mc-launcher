<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getConfig, setConfig, scanJava, type JavaInfo } from "@/services/ipc";
import { useI18n, languages } from "@/i18n";
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
    </section>

    <div class="actions">
      <button class="btn primary" @click="save">{{ t("settings.save") }}</button>
      <span v-if="saved" class="ok">{{ t("settings.saved") }}</span>
    </div>
  </div>
</template>

<style scoped>
.page {
  padding: 34px 40px;
  overflow: auto;
  height: 100%;
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
  border: 1px solid var(--line-2);
  border-radius: var(--radius);
  background: var(--card);
  backdrop-filter: blur(18px) saturate(180%);
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
</style>
