<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { readInstanceLog } from "@/services/ipc";
import type { LogView } from "@/types";

const props = defineProps<{ instanceId: string; title: string }>();
const emit = defineEmits<{ (e: "close"): void }>();

const log = ref<LogView | null>(null);
const loading = ref(true);
const filter = ref("");

async function load() {
  loading.value = true;
  log.value = await readInstanceLog(props.instanceId, 800).catch(() => null);
  loading.value = false;
}

onMounted(load);
watch(
  () => props.instanceId,
  () => load(),
);

const shown = computed(() => {
  const lines = log.value?.lines ?? [];
  const q = filter.value.trim().toLowerCase();
  if (!q) return lines;
  return lines.filter((l) => l.toLowerCase().includes(q));
});

function fmtSize(b: number) {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}
</script>

<template>
  <div class="modal" @click.self="emit('close')">
    <div class="panel">
      <header>
        <div class="title">
          <span class="dot"></span>
          {{ title }}
        </div>
        <div class="tools">
          <input v-model="filter" class="filter mono" placeholder="过滤关键字…" spellcheck="false" />
          <button class="mini" @click="load">刷新</button>
          <button class="mini" @click="emit('close')">关闭</button>
        </div>
      </header>

      <div v-if="loading" class="state">正在读取…</div>
      <div v-else-if="!log?.exists" class="state">
        还没有日志 —— 这个实例还没启动过
      </div>
      <pre v-else class="body mono">{{ shown.join("\n") }}</pre>

      <footer class="foot mono">
        <span v-if="log?.exists">{{ log.path }}</span>
        <span v-else>—</span>
        <span class="right">
          {{ shown.length }} 行<template v-if="log?.truncated"> · 仅显示尾部</template>
          <template v-if="log?.size"> · {{ fmtSize(log.size) }}</template>
        </span>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.modal {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(3px);
  padding: 40px;
}

.panel {
  width: min(980px, 100%);
  height: min(760px, 100%);
  display: flex;
  flex-direction: column;
  border-radius: 14px;
  background: var(--bg);
  border: 1px solid var(--line);
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.35);
  overflow: hidden;
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--line);
  flex-wrap: wrap;
}

.title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--ink);
}

.dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--ink-3);
}

.tools {
  display: flex;
  align-items: center;
  gap: 6px;
}

.filter {
  width: 170px;
  padding: 4px 9px;
  font-size: 11px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: var(--card-hover);
  color: var(--ink);
  outline: none;
}

.filter:focus {
  border-color: var(--ink-3);
}

.body {
  flex: 1;
  margin: 0;
  padding: 12px 16px;
  overflow: auto;
  font-size: 11px;
  line-height: 1.65;
  color: var(--ink-2);
  white-space: pre-wrap;
  word-break: break-all;
}

.state {
  flex: 1;
  display: grid;
  place-items: center;
  font-size: 12px;
  color: var(--ink-3);
}

.foot {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 16px;
  border-top: 1px solid var(--line);
  font-size: 10px;
  color: var(--ink-3);
  word-break: break-all;
}

.right {
  white-space: nowrap;
  flex-shrink: 0;
}

.mini {
  padding: 4px 10px;
  font-size: 11px;
  border: 1px solid var(--line);
  border-radius: 7px;
  background: transparent;
  color: var(--ink-2);
  cursor: pointer;
  white-space: nowrap;
}

.mini:hover {
  background: var(--card-hover);
}
</style>
