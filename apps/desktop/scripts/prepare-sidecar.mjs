import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const host = execFileSync("rustc", ["-vV"], { encoding: "utf8" })
  .match(/^host: (.+)$/m)?.[1]
  .trim();
const target = process.env.TAURI_ENV_TARGET_TRIPLE || host;
if (target !== "x86_64-pc-windows-msvc")
  throw new Error(
    `Desktop currently verified for x86_64-pc-windows-msvc only; got ${target}`,
  );
execFileSync(
  "cargo",
  [
    "build",
    "--manifest-path",
    path.join(root, "Cargo.toml"),
    "-p",
    "skysheet-cli",
    "--release",
    "--locked",
    "--target",
    target,
  ],
  { stdio: "inherit" },
);
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    [
      "metadata",
      "--manifest-path",
      path.join(root, "Cargo.toml"),
      "--no-deps",
      "--format-version",
      "1",
    ],
    { encoding: "utf8" },
  ),
);
const destination = fileURLToPath(
  new URL("../src-tauri/binaries/", import.meta.url),
);
mkdirSync(destination, { recursive: true });
copyFileSync(
  path.join(metadata.target_directory, target, "release", "skysheet.exe"),
  path.join(destination, `skysheet-${target}.exe`),
);
console.log(`Prepared skysheet sidecar (${target})`);
