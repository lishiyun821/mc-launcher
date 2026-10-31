// 根据 Tauri 构建产物生成 updater 需要的 latest.json
//
// Tauri updater 靠这个文件判断有没有新版、去哪下、签名对不对。
// 放在 Release 的 latest 里（releases/latest/download/latest.json），
// 客户端 endpoint 就指这个地址。
//
// 用法：在 release workflow 里构建完之后跑
//   node scripts/make-latest-json.mjs
// 需要环境变量 GITHUB_REF_NAME（= tag 名）

import fs from "node:fs";
import path from "node:path";

const NSIS_DIR = "src-tauri/target/release/bundle/nsis";
const REPO = "lishiyun821/mc-launcher";

const tag = process.env.GITHUB_REF_NAME || process.argv[2];
if (!tag) {
  console.error("缺少 tag：设置 GITHUB_REF_NAME 或作为第一个参数传入");
  process.exit(1);
}

if (!fs.existsSync(NSIS_DIR)) {
  console.error(`找不到目录 ${NSIS_DIR}，是不是还没 build？`);
  process.exit(1);
}

const files = fs.readdirSync(NSIS_DIR);
const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
const ver = pkg.version;

/**
 * 目录里可能残留旧版本的产物（本地开发尤其常见），
 * 所以优先挑文件名里带当前版本号的，找不到再按修改时间取最新。
 */
function pick(ext) {
  const all = files.filter((f) => f.toLowerCase().endsWith(ext));
  if (all.length === 0) return null;
  return (
    all.find((f) => f.includes(ver)) ??
    all
      .map((f) => ({ f, t: fs.statSync(path.join(NSIS_DIR, f)).mtimeMs }))
      .sort((a, b) => b.t - a.t)[0].f
  );
}

const exe = pick(".exe");
const sig = pick(".exe.sig");

if (!exe) {
  console.error("没找到安装包（*.exe）");
  process.exit(1);
}
if (!sig) {
  console.error("没找到签名文件（*.exe.sig）—— 检查 TAURI_SIGNING_PRIVATE_KEY 是否设置");
  process.exit(1);
}

// 发布说明：优先用该 tag 的说明文件，没有就留空
const notesCandidates = [
  `docs/release-notes-${tag}.md`,
  `docs/release-notes-v${tag.replace(/^v/, "")}.md`,
  "docs/release-notes.md",
];
let notes = "";
for (const c of notesCandidates) {
  if (fs.existsSync(c)) {
    notes = fs.readFileSync(c, "utf8").slice(0, 4000);
    break;
  }
}

/**
 * GitHub 储存 Release 附件时会把文件名里的空格换成点
 * （本地是 "MC Launcher_0.1.2_x64-setup.exe"，下载地址是 "MC.Launcher_0.1.2_x64-setup.exe"），
 * 这里提前对齐，否则客户端拿到的 URL 会 404。
 */
const assetName = exe.replace(/ /g, ".");
const url = `https://github.com/${REPO}/releases/download/${tag}/${encodeURIComponent(assetName)}`;

const payload = {
  version: pkg.version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature: fs.readFileSync(path.join(NSIS_DIR, sig), "utf8").trim(),
      url,
    },
  },
};

fs.writeFileSync("latest.json", JSON.stringify(payload, null, 2), "utf8");

console.log("✓ latest.json 已生成");
console.log(`  版本: ${payload.version}`);
console.log(`  安装包: ${exe}`);
console.log(`  附件名: ${assetName}`);
console.log(`  签名: ${sig}`);
console.log(`  地址: ${url}`);
