import { existsSync } from "node:fs";
import { delimiter, dirname, join } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const env = { ...process.env };
const localCargo = join(root, ".tools", "cargo");
const executable = process.platform === "win32" ? "cargo.exe" : "cargo";
if (!env.CARGO_HOME && existsSync(join(localCargo, "bin", executable))) {
  env.CARGO_HOME = localCargo;
  env.RUSTUP_HOME = join(root, ".tools", "rustup");
  // Windows environment names are case-insensitive. Keep only one PATH spelling.
  const pathKey = Object.keys(env).find((key) => key.toUpperCase() === "PATH") ?? "PATH";
  env[pathKey] = `${join(localCargo, "bin")}${delimiter}${env[pathKey] ?? ""}`;
}

const args = process.argv.slice(2);
const cargo = args[0] === "--cargo";
const command = cargo ? (env.CARGO_HOME ? join(env.CARGO_HOME, "bin", executable) : executable) : process.execPath;
const commandArgs = cargo ? args.slice(1) : [join(root, "node_modules", "@tauri-apps", "cli", "tauri.js"), ...args];
const child = spawn(command, commandArgs, { cwd: root, env, stdio: "inherit", windowsHide: true });
child.on("error", (error) => { console.error(`无法启动构建工具：${error.message}\n请先按照 README 安装开发依赖。`); process.exitCode = 1; });
child.on("exit", (code) => { process.exitCode = code ?? 1; });
