<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getAppVersion } from "@/services/ipc";

const emit = defineEmits<{ goto: [page: "home" | "versions" | "settings"] }>();

const appVersion = ref("—");
const cards = [
  { title: "版本", desc: "安装、管理与启动游戏", page: "versions" as const, metric: "0", unit: "个实例" },
  { title: "设置", desc: "下载源、语言与偏好", page: "settings" as const, metric: "官方", unit: "下载源" },
];

onMounted(async () => {
  appVersion.value = await getAppVersion().catch(() => "—");
});
</script>

<template>
  <div class="page">
    <header>
      <div class="eyebrow">OVERVIEW</div>
      <h1>概览</h1>
      <p class="sub">一个开源的 Minecraft 启动器 —— 分层架构 · 任务调度 · 多源下载</p>
    </header>

    <div class="grid">
      <button
        v-for="c in cards"
        :key="c.title"
        class="card"
        @click="emit('goto', c.page)"
      >
        <div class="card-top">
          <span class="card-title">{{ c.title }}</span>
          <span class="card-metric num">{{ c.metric }}<small>{{ c.unit }}</small></span>
        </div>
        <div class="card-desc">{{ c.desc }}</div>
        <div class="card-arrow">→</div>
      </button>
    </div>

    <section class="meta">
      <div class="meta-row">
        <span class="k">应用版本</span>
        <span class="v mono">{{ appVersion }}</span>
      </div>
      <div class="meta-row">
        <span class="k">架构</span>
        <span class="v mono">Core / Service / Infra / UI</span>
      </div>
      <div class="meta-row">
        <span class="k">阶段</span>
        <span class="v">Phase 0 —— 骨架</span>
      </div>
    </section>
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
  line-height: 1.15;
}

.sub {
  margin-top: 7px;
  font-size: 12.5px;
  color: var(--ink-3);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(238px, 1fr));
  gap: 12px;
  margin-top: 26px;
}

.card {
  position: relative;
  text-align: left;
  padding: 15px 16px 17px;
  border: 1px solid var(--line-2);
  border-radius: var(--radius);
  background: var(--card);
  backdrop-filter: blur(18px) saturate(180%);
  box-shadow: var(--shadow);
  transition: background 0.16s ease, transform 0.16s ease, box-shadow 0.16s ease;
}

.card:hover {
  background: var(--card-hover);
  transform: translateY(-1px);
  box-shadow: var(--shadow-lg);
}

.card-top {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
}

.card-title {
  font-size: 13.5px;
  font-weight: 600;
}

.card-metric {
  font-size: 15px;
  color: var(--ink-2);
}

.card-desc {
  margin-top: 8px;
  font-size: 11.5px;
  color: var(--ink-3);
}

.card-arrow {
  position: absolute;
  right: 15px;
  bottom: 13px;
  font-size: 13px;
  color: var(--ink-4);
  transition: transform 0.16s ease, color 0.16s;
}

.card:hover .card-arrow {
  color: var(--ink);
  transform: translateX(2px);
}

.meta {
  margin-top: 30px;
  border-top: 1px solid var(--line-2);
  padding-top: 14px;
  display: flex;
  flex-direction: column;
  gap: 7px;
  max-width: 460px;
}

.meta-row {
  display: flex;
  justify-content: space-between;
  font-size: 11.5px;
}

.meta-row .k {
  color: var(--ink-3);
}

.meta-row .v {
  color: var(--ink-2);
}
</style>
