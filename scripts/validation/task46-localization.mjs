import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(process.argv[2] ?? process.cwd());
const source = readFileSync(resolve(root, "src/i18n/index.tsx"), "utf8").replace(/\r\n/g, "\n");

function localeBlock(locale, nextMarker) {
  const start = source.indexOf(`  ${locale}: {`);
  const end = source.indexOf(nextMarker, start);
  if (start < 0 || end < 0) throw new Error(`Missing locale block: ${locale}`);
  return source.slice(start, end);
}

function keys(block) {
  return new Set([...block.matchAll(/^\s+"([^"]+)":/gm)].map((match) => match[1]));
}

const en = keys(localeBlock("en", "  vi: {"));
const vi = keys(localeBlock("vi", "};\n\nconst localeStorageKey"));
const missingInVi = [...en].filter((key) => !vi.has(key));
const missingInEn = [...vi].filter((key) => !en.has(key));
if (missingInVi.length || missingInEn.length) {
  throw new Error(
    `Locale key drift: missing in vi=[${missingInVi.join(", ")}] missing in en=[${missingInEn.join(", ")}]`,
  );
}

const required = [
  "common.settings",
  "settings.languageDescription",
  "editor.reviewRequired",
  "editor.editTranscript",
  "editor.revertTranscript",
  "editor.revert",
  "settings.telemetryOptIn",
];
for (const key of required) {
  if (!en.has(key) || !vi.has(key)) throw new Error(`Required translation key missing: ${key}`);
}

const forbiddenLiterals = [
  ["src/components/editor/SuggestionCard.tsx", "Review required — segment timing"],
  ["src/components/editor/transcript/TranscriptSegment.tsx", "Double click to edit"],
  ["src/components/editor/transcript/TranscriptSegment.tsx", "Revert transcript segment"],
];
for (const [relativePath, literal] of forbiddenLiterals) {
  const content = readFileSync(resolve(root, relativePath), "utf8");
  if (content.includes(literal)) throw new Error(`Hard-coded UI literal remains: ${relativePath}: ${literal}`);
}

console.log(`Task46 localization validation PASS (${en.size} EN keys, ${vi.size} VI keys)`);
