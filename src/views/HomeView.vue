<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getAppVersion, getConfig, listLocalVersions } from "@/services/ipc";
import { useI18n } from "@/i18n";

const emit = defineEmits<{ goto: [page: "home" | "versions" | "settings"] }>();
const { t } = useI18n();

const appVersion = ref("—");
const localCount = ref(0);
const sourceLabel = ref("—");

const cards = computed(() => [
  {
    title: t("home.cardVersions"),
    desc: t("home.cardVersionsDesc"),
    page: "versions" as const,
    metric: String(localCount.value),
    unit: t("home.cardVersionsUnit"),
  },
  {
    title: t("home.cardSettings"),
    desc: t("home.cardSettingsDesc"),
    page: "settings" as const,
    metric: sourceLabel.value,
    unit: "",
  },
]);

onMounted(async () => {
  appVersion.value = await getAppVersion().catch(() => "—");
  const local = await listLocalVersions().catch(() => []);
  localCount.value = local.length;
  const cfg = await getConfig().catch(() => null);
  sourceLabel.value =
    cfg?.downloadSource === "mirror" ? "BMCLAPI" : "mojang";
});
</script>

<template>
  <div class="page">
    <header>
      <div class="eyebrow">{{ t("home.eyebrow") }}</div>
      <h1>{{ t("home.title") }}</h1>
      <p class="sub">{{ t("home.sub") }}</p>
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
        <span class="k">{{ t("home.metaVersion") }}</span>
        <span class="v mono">{{ appVersion }}</span>
      </div>
      <div class="meta-row">
        <span class="k">{{ t("home.metaArch") }}</span>
        <span class="v mono">Core / Service / Infra / UI</span>
      </div>
      <div class="meta-row">
        <span class="k">{{ t("home.metaStage") }}</span>
        <span class="v">{{ t("home.metaStageValue") }}</span>
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
