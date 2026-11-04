/**
 * Assemble the browser extension packages.
 *
 * The JavaScript and markup are shared; only the manifest differs (Chromium
 * wants a service worker and a fixed key, Firefox wants an event page and a
 * gecko id). Running this produces `dist/chrome` and `dist/firefox`, each ready
 * to load unpacked or to zip for the stores.
 *
 *   node extensions/build.mjs
 */

import { cp, mkdir, readdir, rm, stat } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const shared = join(here, "shared");
const browsers = ["chrome", "firefox"];

/** Shared assets copied verbatim into every package. */
const SHARED_FILES = [
  "background.js",
  "content.js",
  "popup.html",
  "popup.js",
  "styles.css",
  "icons",
];

async function exists(path) {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

async function build(browser) {
  const target = join(here, "dist", browser);
  await rm(target, { recursive: true, force: true });
  await mkdir(target, { recursive: true });

  for (const entry of SHARED_FILES) {
    const source = join(shared, entry);
    if (!(await exists(source))) {
      throw new Error(`missing shared asset: ${entry}`);
    }
    await cp(source, join(target, entry), { recursive: true });
  }

  await cp(join(here, browser, "manifest.json"), join(target, "manifest.json"));

  // A manifest that points at a file the package does not contain is the most
  // common way an extension fails to load, so verify every referenced path.
  const manifest = JSON.parse(
    await import("node:fs/promises").then((fs) =>
      fs.readFile(join(target, "manifest.json"), "utf8"),
    ),
  );

  const referenced = new Set();
  if (manifest.background?.service_worker) referenced.add(manifest.background.service_worker);
  for (const script of manifest.background?.scripts ?? []) referenced.add(script);
  for (const script of manifest.content_scripts ?? []) {
    for (const file of [...(script.js ?? []), ...(script.css ?? [])]) referenced.add(file);
  }
  for (const file of Object.values(manifest.icons ?? {})) referenced.add(file);
  if (manifest.action?.default_popup) referenced.add(manifest.action.default_popup);

  for (const file of referenced) {
    if (!(await exists(join(target, file)))) {
      throw new Error(`${browser}: manifest references a missing file: ${file}`);
    }
  }

  const files = await readdir(target);
  console.log(`  ${browser.padEnd(8)} -> extensions/dist/${browser}  (${files.length} entries)`);
}

console.log("Building Vidaro extensions…");
for (const browser of browsers) {
  await build(browser);
}
console.log("Done. Load `extensions/dist/chrome` as an unpacked extension, or");
console.log("`extensions/dist/firefox` via about:debugging → Load Temporary Add-on.");
