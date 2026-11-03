// Builds a new version of Rokhsa on THIS PC and uploads it to GitHub as a
// DRAFT release. Schools see nothing until the owner opens the draft on
// GitHub and clicks Publish.
//
//   npm run release            build, check, upload a draft
//   npm run release -- --dry   build and check only, upload nothing
//
// Two installers come out of it, and both install WebView2 only if the PC lacks it:
//   Rokhsa_<v>_x64-setup.exe   ~3 MB. What installed copies download to update.
//                              Fetches WebView2 from the internet when missing.
//   Rokhsa_<v>_x64-usb.exe     ~210 MB. What goes on the USB stick for a new school.
//                              Carries WebView2 inside, so it installs with no
//                              internet (some "lite" Windows copies strip it out).
//                              Also copied to the Desktop.
//
// Built here, not on GitHub, on purpose: the update signing key and the
// unlock code's hash never leave the owner's PC.
//
// Needs, all on this PC:
//   %USERPROFILE%\.tauri\rokhsa-updater.key   the update signing key
//   src-tauri\unlock.hash                     the unlock code's hash (gitignored)
//   gh, signed in once                        (gh auth login --web)
//
// Before running: raise "version" in src-tauri/tauri.conf.json and
// package.json (they must agree) and commit everything.
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const REPO = "Sidahmed-N/rokhsa";
const dry = process.argv.includes("--dry");
const root = path.resolve(path.dirname(new URL(import.meta.url).pathname.replace(/^\/(\w:)/, "$1")), "..");

function fail(message) {
  console.error(`\n✗ ${message}\n`);
  process.exit(1);
}
function run(cmd, args, opts = {}) {
  return execFileSync(cmd, args, { cwd: root, stdio: "inherit", shell: process.platform === "win32", ...opts });
}
function read(cmd, args) {
  return execFileSync(cmd, args, { cwd: root, encoding: "utf8", shell: process.platform === "win32" }).trim();
}
function gh(args, opts = {}) {
  const exe = fs.existsSync("C:\\Program Files\\GitHub CLI\\gh.exe")
    ? "C:\\Program Files\\GitHub CLI\\gh.exe"
    : "gh";
  return execFileSync(exe, args, { cwd: root, encoding: "utf8", ...opts });
}

// 1. Everything the build needs is here, and the versions agree.
const conf = JSON.parse(fs.readFileSync(path.join(root, "src-tauri/tauri.conf.json"), "utf8"));
const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const version = conf.version;
if (pkg.version !== version) fail(`Versions disagree: tauri.conf.json ${version}, package.json ${pkg.version}.`);
const tag = `v${version}`;

const key = path.join(os.homedir(), ".tauri", "rokhsa-updater.key");
if (!fs.existsSync(key)) fail(`Signing key not found: ${key}`);
if (!fs.existsSync(path.join(root, "src-tauri/unlock.hash")))
  fail("src-tauri/unlock.hash not found: the app would have no lock.");

// A release must be exactly a commit on GitHub; a dry run may try anything.
if (!dry && read("git", ["status", "--porcelain"]) !== "") fail("Uncommitted changes: commit them first.");

if (!dry) {
  try {
    gh(["auth", "status"], { stdio: "pipe" });
  } catch {
    fail("GitHub CLI is not signed in. Run: gh auth login --web");
  }
  let exists = false;
  try {
    gh(["release", "view", tag, "--repo", REPO], { stdio: "pipe" });
    exists = true;
  } catch {
    // No release with this tag: good.
  }
  if (exists) fail(`A release ${tag} already exists. Raise the version first.`);
}

// 2. The same gate as every commit, then the Rust tests.
console.log(`\n▶ Checking ${tag}…`);
run("npm", ["run", "check"]);
run("cargo", ["test", "--lib"], { cwd: path.join(root, "src-tauri") });

// 3. Build and sign the installers.
// cargo's target dir may be redirected (src-tauri/.cargo/config.toml). Asked
// from inside src-tauri: that config only applies there, and asked from the
// root cargo answered the default folder, where no installer was.
let targetDir = path.join(root, "src-tauri", "target");
try {
  const out = execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
    cwd: path.join(root, "src-tauri"),
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  targetDir = JSON.parse(out).target_directory;
} catch {
  // Keep the default.
}
const bundle = path.join(targetDir, "release", "bundle", "nsis");
const installerName = `${conf.productName}_${version}_x64-setup.exe`;
const installer = path.join(bundle, installerName);
const usbName = `${conf.productName}_${version}_x64-usb.exe`;
const usbInstaller = path.join(bundle, usbName);
const signing = { ...process.env, TAURI_SIGNING_PRIVATE_KEY: key, TAURI_SIGNING_PRIVATE_KEY_PASSWORD: "" };

// The USB one first, set aside under its own name: both builds write the same file.
console.log(`\n▶ Building ${tag} for USB (WebView2 inside)…`);
run("npx", ["tauri", "build", "--config", "src-tauri/tauri.usb.conf.json"], { env: signing });
if (!fs.existsSync(installer)) fail(`USB installer missing in ${bundle}`);
fs.renameSync(installer, usbInstaller);
fs.rmSync(`${installer}.sig`, { force: true });

console.log(`\n▶ Building ${tag} for updates…`);
run("npx", ["tauri", "build"], { env: signing });
const signatureFile = `${installer}.sig`;
if (!fs.existsSync(installer) || !fs.existsSync(signatureFile))
  fail(`Installer or signature missing in ${bundle}`);

// 4. latest.json: what installed apps read to learn there is a new version.
const url = `https://github.com/${REPO}/releases/download/${tag}/${installerName.replace(/ /g, ".")}`;
const latest = {
  version,
  notes: `Rokhsa ${version}`,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": { signature: fs.readFileSync(signatureFile, "utf8").trim(), url },
  },
};
const latestFile = path.join(bundle, "latest.json");
fs.writeFileSync(latestFile, JSON.stringify(latest, null, 2));
const mb = (file) => `${(fs.statSync(file).size / 1048576).toFixed(2)} MB`;
console.log(`\n✓ Built ${installerName} (${mb(installer)}), signed: for updates.`);
console.log(`✓ Built ${usbName} (${mb(usbInstaller)}): for USB.`);
const desktop = path.join(os.homedir(), "Desktop");
if (fs.existsSync(desktop)) {
  fs.copyFileSync(usbInstaller, path.join(desktop, usbName));
  console.log(`  Copied to ${path.join(desktop, usbName)}`);
}

if (dry) {
  console.log(`\n(dry run) Nothing uploaded. Files in ${bundle}`);
  process.exit(0);
}

// 5. Upload as a draft. Tagged at the current main commit when published.
console.log(`\n▶ Uploading draft ${tag}…`);
gh(
  [
    "release", "create", tag,
    "--repo", REPO,
    "--draft",
    "--target", read("git", ["rev-parse", "HEAD"]),
    "--title", `Rokhsa ${version}`,
    "--notes", "Nouvelle version de Rokhsa.",
    installer, signatureFile, latestFile, usbInstaller,
  ],
  { stdio: "inherit" },
);
console.log(`\n✓ Draft ${tag} is on GitHub. Schools see it only after you click Publish:`);
console.log(`  https://github.com/${REPO}/releases\n`);
