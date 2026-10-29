<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getConfig, setConfig, scanJava, type JavaInfo } from "@/services/ipc";
import type { AppConfig } from "@/types";

const config = ref<AppConfig | null>(null);
const javas = ref<JavaInfo[]>([]);
const saved = ref(false);

onMounted(async () => {
  config.value = await getConfig().catch(() => null);
  javas.value = await scanJava().catch(() => []);
});

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
      <div class="eyebrow">SETTINGS</div>
      <h1>设置</h1>
      <p class="sub">下载源、语言与运行环境</p>
    </header>

    <section v-if="config" class="group">
      <div class="group-title">下载源</div>
      <div class="row">
        <div class="row-main">
          <div class="label">优先下载源</div>
          <div class="desc">镜像源失败时会自动降级到官方源，反之亦然</div>
        </div>
        <select v-model="config.downloadSource" class="select">
          <option value="official">官方源（mojang）</option>
          <option value="mirror">镜像源（BMCLAPI）</option>
        </select>
      </div>
      <div class="row">
        <div class="row-main">
          <div class="label">语言</div>
          <div class="desc">界面语言</div>
        </div>
        <select v-model="config.language" class="select">
          <option value="zh-CN">简体中文</option>
          <option value="en">English</option>
        </select>
      </div>
    </section>

    <section class="group">
      <div class="group-title">Java 运行环境</div>
      <div v-if="!javas.length" class="row">
        <div class="row-main">
          <div class="label">未检测到 Java</div>
          <div class="desc">安装 JDK 后重启应用，或手动指定 java.exe 路径</div>
        </div>
      </div>
      <div v-for="j in javas" :key="j.path" class="row">
        <div class="row-main">
          <div class="label mono">{{ j.version }}</div>
          <div class="desc mono">{{ j.path }}</div>
        </div>
        <span class="badge">{{ j.major }}</span>
        <span v-if="j.is64Bit" class="badge">64 位</span>
      </div>
    </section>

    <div class="actions">
      <button class="btn primary" @click="save">保存设置</button>
      <span v-if="saved" class="ok">已保存</span>
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
