<script setup lang="ts">
/**
 * 玩家的方块头头像。
 *
 * 首选自己从官方皮肤图（textures.minecraft.net）里切出头部：
 * 皮肤是 64x64 / 64x32 的展开图，头部在 (8,8) 8x8，帽子层在 (40,8) 8x8。
 * 官方 CDN 带 `Access-Control-Allow-Origin: *`，所以前端 canvas 切图可用，
 * 不依赖任何第三方头像服务。
 *
 * 拿不到皮肤（离线账户 / 断网）时依次回退：
 *   Crafatar → mc-heads → 首字母色块
 */
import { computed, onMounted, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    uuid?: string | null;
    name?: string | null;
    skinUrl?: string | null;
    size?: number;
  }>(),
  { uuid: "", name: "", skinUrl: null, size: 64 },
);

/** 本地切出来的头像（data URL） */
const dataUrl = ref<string | null>(null);
/** 第三方源的序号 */
const fallbackIndex = ref(0);
/** 全都失败 */
const failed = ref(false);

const id = computed(() => (props.uuid || "").replace(/-/g, "").toLowerCase());

const fallbacks = computed<string[]>(() => {
  if (!id.value) return [];
  const s = props.size;
  return [
    `https://crafatar.com/avatars/${id.value}?size=${s}&overlay`,
    `https://mc-heads.net/avatar/${id.value}/${s}`,
  ];
});

const src = computed(() => {
  if (dataUrl.value) return dataUrl.value;
  return fallbacks.value[fallbackIndex.value] ?? "";
});

const initial = computed(() => (props.name || "?").slice(0, 1).toUpperCase());
const box = computed(() => ({ width: `${props.size}px`, height: `${props.size}px` }));

/** 从皮肤展开图里切出头部方块图 */
async function sliceHead(url: string): Promise<string | null> {
  const img = new Image();
  img.crossOrigin = "anonymous";
  img.decoding = "sync";
  img.src = url.replace(/^http:/, "https:");

  await new Promise<void>((resolve, reject) => {
    img.onload = () => resolve();
    img.onerror = () => reject(new Error("skin load failed"));
  });

  const w = img.naturalWidth || img.width;
  const h = img.naturalHeight || img.height;
  if (!w || !h) return null;

  // 皮肤是 64 宽的展开图；容错按比例换算
  const unit = w / 64;

  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;

  ctx.imageSmoothingEnabled = false;

  // 底层头部：(8,8) 8x8
  ctx.drawImage(img, 8 * unit, 8 * unit, 8 * unit, 8 * unit, 0, 0, 64, 64);

  // 帽子层：(40,8) 8x8，叠在上面
  if (w >= 64) {
    ctx.drawImage(img, 40 * unit, 8 * unit, 8 * unit, 8 * unit, 0, 0, 64, 64);
  }

  try {
    return canvas.toDataURL("image/png");
  } catch {
    // 极端情况下 canvas 被标记为污染
    return null;
  }
}

async function refresh() {
  dataUrl.value = null;
  fallbackIndex.value = 0;
  failed.value = false;

  const url = props.skinUrl;
  if (!url) return;

  try {
    const cut = await sliceHead(url);
    if (cut) dataUrl.value = cut;
  } catch {
    // 切图失败就走第三方回退
  }
}

function onError() {
  if (dataUrl.value) {
    // 切图结果异常，退回第三方
    dataUrl.value = null;
    return;
  }
  if (fallbackIndex.value < fallbacks.value.length - 1) fallbackIndex.value += 1;
  else failed.value = true;
}

onMounted(refresh);
watch(() => [props.skinUrl, props.uuid], refresh);
</script>

<template>
  <img
    v-if="src && !failed"
    class="skin-avatar"
    :src="src"
    :alt="name ?? ''"
    :style="box"
    draggable="false"
    @error="onError"
  />
  <span v-else class="skin-avatar fallback" :style="box">{{ initial }}</span>
</template>

<style scoped>
.skin-avatar {
  flex: none;
  display: block;
  border-radius: 22%;
  object-fit: cover;
  background: var(--ink-4);
  /* 皮肤是像素图，放大后保持硬边 */
  image-rendering: pixelated;
  image-rendering: crisp-edges;
}

.skin-avatar.fallback {
  display: grid;
  place-items: center;
  font-size: 13px;
  font-weight: 600;
  color: var(--bg);
  background: var(--ink);
}
</style>
