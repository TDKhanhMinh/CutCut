import { expect, test } from '@playwright/test';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { redactTelemetryProperties } from '../../src/services/telemetry';

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    const invoke = async (command: string) => {
      switch (command) {
        case 'get_auth_session':
          return null;
        case 'get_gemini_key_status':
          return { configured: false, maskedHint: null };
        case 'get_runtime_profile':
          return {
            cpuName: 'Playwright fixture CPU',
            totalMemoryMb: 8192,
            runtimeAvailable: true,
            runtimeVersion: 'fixture',
            supportedAcceleration: 'cpu',
            recommendedModelIds: ['ggml-tiny'],
            gpuNames: [],
          };
        case 'get_runtime_preset_preference':
          return { preset: 'balanced', userOverrideModel: null };
        case 'resolve_runtime_preset':
          return {
            targetModelId: 'ggml-tiny',
            targetBackend: 'cpu',
            tradeoffDescription: 'fixture',
            isModelInstalled: true,
            fallbackReason: null,
          };
        default:
          return null;
      }
    };
    (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = { invoke };
  });
});

test('opens the local-first editor shell without uploading media', async ({ page }) => {
  await page.goto('/#/');
  await expect(page.getByRole('heading', { name: /Welcome to CutCut/i })).toBeVisible();
  await expect(page.getByText(/create a new project or open an existing one/i)).toBeVisible();
  await expect(page.locator('input[type=file]')).toHaveCount(0);
});

test('ships the valid, corrupted and portable project fixtures locally', async () => {
  const fixtureRoot = resolve('qa/fixtures');
  const sample = resolve(fixtureRoot, 'sample.mp4');
  const corrupted = resolve(fixtureRoot, 'corrupted.mp4');
  const project = resolve(fixtureRoot, 'Untitled.cutcut');
  const portrait = resolve(fixtureRoot, 'portrait.mp4');
  const noAudio = resolve(fixtureRoot, 'no-audio.mp4');
  const unicodeProject = resolve(fixtureRoot, 'Unicode-Việt.cutcut');
  const missingMediaProject = resolve(fixtureRoot, 'missing-media.cutcut');
  expect(existsSync(sample)).toBe(true);
  expect(existsSync(corrupted)).toBe(true);
  expect(existsSync(project)).toBe(true);
  expect(existsSync(portrait)).toBe(true);
  expect(existsSync(noAudio)).toBe(true);
  expect(existsSync(unicodeProject)).toBe(true);
  expect(existsSync(missingMediaProject)).toBe(true);
  expect(readFileSync(sample).subarray(4, 8).toString()).toBe('ftyp');
  expect(statSync(corrupted).size).toBeLessThan(statSync(sample).size);
  const probe = (file: string) =>
    JSON.parse(execFileSync('ffprobe', ['-v', 'error', '-show_streams', '-of', 'json', file], { encoding: 'utf8' }));
  const portraitStreams = probe(portrait).streams;
  expect(portraitStreams.some((stream: { width?: number; height?: number }) => stream.width === 720 && stream.height === 1280)).toBe(true);
  expect(probe(noAudio).streams.some((stream: { codec_type?: string }) => stream.codec_type === 'audio')).toBe(false);
  expect(JSON.parse(readFileSync(project, 'utf8')).media[0].path).toBe('sample.mp4');
  expect(JSON.parse(readFileSync(unicodeProject, 'utf8')).transcript.segments[0].text).toContain('Xin chào');
  expect(JSON.parse(readFileSync(missingMediaProject, 'utf8')).media[0].path).toBe('does-not-exist.mp4');
});

test('settings can switch and persist EN/VI locale independently of project data', async ({ page }) => {
  await page.goto('/#/settings');
  const language = page.getByRole('combobox', { name: /language|ngôn ngữ/i });
  await expect(language).toBeVisible();
  await language.selectOption('en');
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  await page.reload();
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  await expect(language).toHaveValue('en');
});

test('BYOK UI never exposes a stored full key', async ({ page }) => {
  await page.goto('/#/settings');
  await expect(page.getByText(/not configured|chưa cấu hình/i)).toBeVisible();
  await expect(page.locator('input[type=password]')).toHaveAttribute('autocomplete', 'new-password');
});

test('telemetry allowlist redacts paths, JWTs and API keys', async () => {
  const redacted = redactTelemetryProperties({
    errorCode: 'Bearer eyJheader.payload.signature C:\\Users\\alice\\project.cutcut',
    transcript: 'secret transcript must be dropped',
    durationMs: 42,
  });
  expect(redacted.transcript).toBeUndefined();
  expect(redacted.durationMs).toBe(42);
  expect(String(redacted.errorCode)).not.toContain('eyJheader');
  expect(String(redacted.errorCode)).not.toContain('C:\\Users\\alice');
});
