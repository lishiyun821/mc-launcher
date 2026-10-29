import { computed, ref } from "vue";
import zhCN from "./zh-CN";
import en from "./en";

export type Lang = "zh-CN" | "en";

const dicts: Record<Lang, typeof zhCN> = {
  "zh-CN": zhCN,
  en: en as unknown as typeof zhCN,
};

/** 当前语言（响应式） */
export const currentLang = ref<Lang>("zh-CN");

/** 语言选项，给 UI 用 */
export const languages: { value: Lang; label: string }[] = [
  { value: "zh-CN", label: "简体中文" },
  { value: "en", label: "English" },
];

/** 当前语言的完整字典 */
export const dict = computed(() => dicts[currentLang.value]);

/**
 * 取译文。支持点号路径：t("nav.home")
 * 读取 currentLang 会建立依赖，语言一变模板自动重渲染
 */
export function t(path: string): string {
  const parts = path.split(".");
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let node: any = dicts[currentLang.value];
  for (const p of parts) {
    node = node?.[p];
    if (node === undefined) return path;
  }
  return typeof node === "string" ? node : path;
}

/** 切换语言 */
export function setLang(lang: Lang) {
  if (lang in dicts) {
    currentLang.value = lang;
    document.documentElement.lang = lang;
  }
}

/** 供模板直接用的组合函数 */
export function useI18n() {
  return { t, currentLang, setLang, languages, dict };
}
