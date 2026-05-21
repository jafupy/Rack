import { watch } from "fs";
import { spawn, execSync } from "child_process";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

const BUILD_SCRIPT = resolve(__dirname, "build-app.sh");
const APP_PATH = resolve(root, "dist/Rack.app");
const WATCH_DIR = resolve(root, "Sources");

let building = false;
let dirty = false;
let debounce = null;

function kill() {
  try {
    execSync(`pkill -f "Rack.app/Contents/MacOS/Rack"`, { stdio: "ignore" });
  } catch {}
}

function launch() {
  spawn("open", [APP_PATH], { stdio: "ignore" });
}

async function build() {
  if (building) {
    dirty = true;
    return;
  }

  building = true;
  dirty = false;

  console.log("\n🔨 Building...");
  const start = Date.now();

  const proc = spawn("zsh", [BUILD_SCRIPT], { stdio: "inherit" });

  proc.on("close", (code) => {
    building = false;

    if (code !== 0) {
      console.error(`\n❌ Build failed (exit ${code})`);
    } else {
      const elapsed = ((Date.now() - start) / 1000).toFixed(1);
      console.log(`\n✅ Built in ${elapsed}s — relaunching`);
      kill();
      launch();
    }

    if (dirty) build();
  });
}

function onChange(filename) {
  if (!filename) return;
  clearTimeout(debounce);
  debounce = setTimeout(() => {
    console.log(`\n📝 ${filename}`);
    build();
  }, 150);
}

watch(WATCH_DIR, { recursive: true }, (_, filename) => onChange(filename));

console.log(`👀 Watching ${WATCH_DIR}`);
build();
