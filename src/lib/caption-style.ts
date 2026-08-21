import type { CaptionStyle } from "@/types/project";

export const CAPTION_SAFE_AREA = {
  minX: 0.05,
  maxX: 0.95,
  minY: 0.1,
  maxY: 0.95,
  maxWidth: 0.9,
} as const;

export interface CaptionOverlayStyle {
  positionX: number;
  positionY: number;
  fontSize: number;
  fontFamily: string;
  fontWeight: number;
  fontStyle: "normal" | "italic" | "oblique";
  alignment: "left" | "center" | "right";
  primaryColor: string;
  outlineColor: string | null;
  outlineWidth: number;
  backgroundColor: string | null;
}

const clamp = (value: number, min: number, max: number, fallback: number) =>
  Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : fallback;

function normalizeColor(value: string | null | undefined, fallback: string): string {
  const candidate = value?.trim() ?? "";
  if (/^#[0-9a-f]{6}$/i.test(candidate)) return candidate.toUpperCase();
  if (/^[0-9a-f]{6}$/i.test(candidate)) return `#${candidate.toUpperCase()}`;
  return fallback;
}

function withOpacity(color: string, opacity: number | null | undefined): string {
  const alpha = clamp(opacity ?? 1, 0, 1, 1);
  if (alpha >= 1) return color;
  const red = Number.parseInt(color.slice(1, 3), 16);
  const green = Number.parseInt(color.slice(3, 5), 16);
  const blue = Number.parseInt(color.slice(5, 7), 16);
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

function normalizeFontFamily(value: string | null | undefined): string {
  const candidate = value?.trim() ?? "";
  return candidate.length > 0 && candidate.length <= 64 && /^[a-zA-Z0-9 _-]+$/.test(candidate)
    ? candidate
    : "Arial";
}

/**
 * Map the canonical CaptionStyle contract to preview semantics. The same
 * normalized ranges are consumed by the native FFmpeg style mapper, so UI
 * edits do not introduce a second set of positioning rules.
 */
export function mapCaptionStyleToOverlay(style: CaptionStyle): CaptionOverlayStyle {
  const alignment = ["left", "center", "right"].includes(style.alignment)
    ? (style.alignment as CaptionOverlayStyle["alignment"])
    : "center";
  const outlineColor = style.outlineColor ? normalizeColor(style.outlineColor, "#000000") : null;
  const backgroundColor = style.backgroundColor
    ? withOpacity(normalizeColor(style.backgroundColor, "#000000"), style.backgroundOpacity ?? 0.8)
    : null;

  return {
    positionX: clamp(style.positionXVw, CAPTION_SAFE_AREA.minX, CAPTION_SAFE_AREA.maxX, 0.5),
    positionY: clamp(style.positionYVh, CAPTION_SAFE_AREA.minY, CAPTION_SAFE_AREA.maxY, 0.85),
    fontSize: clamp(style.fontSizeVh, 0.01, 0.25, 0.06),
    fontFamily: normalizeFontFamily(style.fontFamily),
    fontWeight: Math.round(clamp(style.fontWeight, 100, 900, 700)),
    fontStyle: ["normal", "italic", "oblique"].includes(style.fontStyle)
      ? (style.fontStyle as CaptionOverlayStyle["fontStyle"])
      : "normal",
    alignment,
    primaryColor: normalizeColor(style.primaryColor, "#FFFFFF"),
    outlineColor,
    outlineWidth: clamp(style.outlineWidthVh ?? 0, 0, 0.05, 0),
    backgroundColor,
  };
}
