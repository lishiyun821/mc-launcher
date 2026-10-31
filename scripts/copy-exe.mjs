// 把编译好的 exe 拷到项目根目录，双击即开（跟 PCL 一样不用找路径）
import { copyFileSync, existsSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = join(root, "src-tauri", "target", "release", "mc-launcher.exe");
const dst = join(root, "MC Launcher.exe");

if (!existsSync(src)) {
  console.error("\n[x] 找不到编译产物：" + src);
  console.error("    先跑一次 npm run build:exe\n");
  process.exit(1);
}

copyFileSync(src, dst);
const mb = (statSync(dst).size / 1024 / 1024).toFixed(2);
console.log("\n✓ 已复制到根目录：MC Launcher.exe (" + mb + " MB)");
