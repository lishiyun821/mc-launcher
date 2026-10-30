/**
 * 主题：system / light / dark
 *
 * system 时不写 data-theme，交给 main.css 里的 prefers-color-scheme 处理；
 * 手动指定时写 data-theme，覆盖系统偏好。
 */
export type Theme = "system" | "light" | "dark";

export function applyTheme(theme: string) {
  const root = document.documentElement;
  if (theme === "light" || theme === "dark") {
    root.setAttribute("data-theme", theme);
  } else {
    root.removeAttribute("data-theme");
  }
}

/** 当前实际生效的是浅色还是深色（给需要判断的地方用） */
export function effectiveTheme(theme: string): "light" | "dark" {
  if (theme === "light" || theme === "dark") return theme;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}
