import { expect, test } from '@playwright/test';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { redactTelemetryProperties } from '../../src/services/telemetry';
import {
  applyCutPreviewDecision,
  buildCutIndex,
  decideCutPreview,
} from '../../src/hooks/useCutPreview';
import type { EditPlan } from '../../src/types/project';
import { mapCaptionStyleToOverlay } from '../../src/lib/caption-style';
import { findActiveCaptionCue, sortCaptionCues } from '../../src/lib/caption-overlay';
import { normalizeEntitlement } from '../../src/lib/entitlements';

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

test('cut preview skips merged enabled ranges without mutating source playback state', () => {
  const plan = {
    actions: [
      { id: 'cut-a', type: 'cut', sourceMediaId: 'media-1', startMs: 1_000, endMs: 2_000, enabled: true },
      { id: 'cut-b', type: 'cut', sourceMediaId: 'media-1', startMs: 1_800, endMs: 3_000, enabled: true },
      { id: 'other-media', type: 'cut', sourceMediaId: 'media-2', startMs: 4_000, endMs: 5_000, enabled: true },
      { id: 'disabled', type: 'cut', sourceMediaId: 'media-1', startMs: 6_000, endMs: 7_000, enabled: false },
    ],
  } as EditPlan;

  const cutIndex = buildCutIndex(plan, 'media-1');
  expect(cutIndex).toEqual([{ startMs: 1_000, endMs: 3_000 }]);

  const firstTick = decideCutPreview(cutIndex, 1_500, 10_000, null);
  expect(firstTick).toEqual({ kind: 'seek', targetMs: 3_000, nextLastSeekTarget: 3_000 });
  const video = { currentTime: 1.5, pause: () => undefined };
  applyCutPreviewDecision(video, firstTick);
  expect(video.currentTime).toBe(3);

  const loopGuardTick = decideCutPreview(cutIndex, 2_950, 10_000, 3_000);
  expect(loopGuardTick).toEqual({ kind: 'noop', nextLastSeekTarget: 3_000 });

  const outsideCut = decideCutPreview(cutIndex, 3_200, 10_000, 3_000);
  expect(outsideCut).toEqual({ kind: 'noop', nextLastSeekTarget: null });

  const endOfMedia = decideCutPreview([{ startMs: 9_000, endMs: 12_000 }], 9_500, 10_000, null);
  expect(endOfMedia).toEqual({ kind: 'pause', targetMs: 10_000, nextLastSeekTarget: null });
  let paused = false;
  const endVideo = { currentTime: 9.5, pause: () => { paused = true; } };
  applyCutPreviewDecision(endVideo, endOfMedia);
  expect(paused).toBe(true);
  expect(endVideo.currentTime).toBe(10);
});

test('caption overlay follows cue timing, Unicode text and canonical style edits', () => {
  const cues = [
    { id: 'late', sourceSegmentIds: [], startMs: 2_000, endMs: 3_000, text: 'Xin chào thế giới', isManualModified: false },
    { id: 'invalid', sourceSegmentIds: [], startMs: 3_000, endMs: 2_000, text: 'ignored', isManualModified: false },
    { id: 'early', sourceSegmentIds: [], startMs: 0, endMs: 1_000, text: 'Việt Nam', isManualModified: false },
  ];
  const sorted = sortCaptionCues(cues);
  expect(sorted.map((cue) => cue.id)).toEqual(['early', 'late']);
  expect(findActiveCaptionCue(sorted, 500)?.text).toBe('Việt Nam');
  expect(findActiveCaptionCue(sorted, 2_500)?.text).toBe('Xin chào thế giới');
  expect(findActiveCaptionCue(sorted, 3_000)).toBeNull();

  const baseStyle = {
    presetId: '16-9',
    fontFamily: 'Arial',
    fontWeight: 700,
    fontStyle: 'normal',
    fontSizeVh: 0.06,
    positionXVw: 0.5,
    positionYVh: 0.85,
    alignment: 'center',
    primaryColor: '#FFFFFF',
    outlineColor: '#000000',
    outlineWidthVh: 0.01,
    backgroundColor: null,
    backgroundOpacity: null,
  } as const;
  const editedStyle = mapCaptionStyleToOverlay({ ...baseStyle, positionXVw: 1.5, alignment: 'right' });
  expect(editedStyle.positionX).toBe(0.95);
  expect(editedStyle.alignment).toBe('right');
  expect(editedStyle.fontFamily).toBe('Arial');
});

test('normalizes the canonical Supabase entitlement schema without unlocking malformed features', () => {
  expect(normalizeEntitlement({
    plan_id: 'pro',
    features: { FEATURE_CLOUD_AI: true, FEATURE_BATCH_EXPORT: false },
    expires_at: '2026-09-01T00:00:00Z',
  })).toEqual({
    plan: 'PRO',
    capabilities: ['FEATURE_CLOUD_AI'],
    expiresAt: '2026-09-01T00:00:00Z',
  });

  expect(normalizeEntitlement({ plan_id: 'admin', features: { bypass: true } })).toEqual({
    plan: 'FREE',
    capabilities: ['bypass'],
    expiresAt: null,
  });
  expect(normalizeEntitlement({ plan_id: 'PRO', features: { capabilities: ['FEATURE_CLOUD_AI', 7] } }))
    .toEqual({ plan: 'PRO', capabilities: ['FEATURE_CLOUD_AI'], expiresAt: null });
});
