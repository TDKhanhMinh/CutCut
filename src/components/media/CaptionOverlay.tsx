import React, { useEffect, useState, useMemo } from "react";
import { CaptionCue, CaptionStyle, EditPlan } from "@/types/project";
import { buildCutIndex, findActiveCut } from "@/hooks/useCutPreview";

interface CaptionOverlayProps {
  videoRef: React.RefObject<HTMLVideoElement | null>;
  cues: CaptionCue[];
  styleModel: CaptionStyle | null;
  editPlan?: EditPlan | null;
  sourceMediaId?: string;
}

export const CaptionOverlay: React.FC<CaptionOverlayProps> = ({
  videoRef,
  cues,
  styleModel,
  editPlan,
  sourceMediaId,
}) => {
  const [currentTimeMs, setCurrentTimeMs] = useState(0);

  // Sync current time at 60fps for smooth caption rendering
  useEffect(() => {
    let handle: number;
    const loop = () => {
      if (videoRef.current) {
        setCurrentTimeMs(videoRef.current.currentTime * 1000);
      }
      handle = requestAnimationFrame(loop);
    };
    handle = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(handle);
  }, [videoRef]);

  // Fast lookup for active cue
  const sortedCues = useMemo(
    () => [...cues].filter((cue) => cue.endMs > cue.startMs).sort((a, b) => a.startMs - b.startMs),
    [cues],
  );

  const activeCue = useMemo(() => {
    let low = 0;
    let high = sortedCues.length - 1;
    while (low <= high) {
      const middle = Math.floor((low + high) / 2);
      const cue = sortedCues[middle];
      if (currentTimeMs < cue.startMs) high = middle - 1;
      else if (currentTimeMs >= cue.endMs) low = middle + 1;
      else return cue;
    }
    return null;
  }, [sortedCues, currentTimeMs]);

  const cutIndex = useMemo(
    () => (editPlan ? buildCutIndex(editPlan, sourceMediaId) : []),
    [editPlan, sourceMediaId],
  );

  // Check if we are inside an enabled cut
  const isInsideCut = useMemo(() => {
    return Boolean(findActiveCut(cutIndex, currentTimeMs));
  }, [cutIndex, currentTimeMs]);

  if (!styleModel || !activeCue || isInsideCut) {
    return null;
  }

  const clamp = (value: number, min: number, max: number) =>
    Number.isFinite(value) ? Math.min(max, Math.max(min, value)) : min;
  const positionX = clamp(styleModel.positionXVw, 0.05, 0.95);
  const positionY = clamp(styleModel.positionYVh, 0.1, 0.95);
  const fontSize = clamp(styleModel.fontSizeVh, 0.01, 0.25);

  // Convert CaptionStyle to React CSS with the same normalized safe-area
  // semantics used by the native mapper. The overlay width is bounded so
  // left/right alignment cannot silently overflow the preview frame.
  // Container query (`@container`) is required on the parent wrapper for `cqh` to work.
  const containerStyle: React.CSSProperties = {
    position: "absolute",
    left: `${positionX * 100}%`,
    top: `${positionY * 100}%`,
    transform: "translate(-50%, -100%)", // Align bottom-center
    width: "90%",
    maxWidth: "90%",
    overflow: "hidden",
    pointerEvents: "none",
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    zIndex: 10,
  };

  // Override transform based on alignment
  if (styleModel.alignment === "left") {
    containerStyle.transform = "translate(0%, -100%)";
    containerStyle.alignItems = "flex-start";
  } else if (styleModel.alignment === "right") {
    containerStyle.transform = "translate(-100%, -100%)";
    containerStyle.alignItems = "flex-end";
  }

  // Map properties
  const textStyle: React.CSSProperties = {
    fontFamily: styleModel.fontFamily?.trim() || "Arial, sans-serif",
    fontWeight: styleModel.fontWeight,
    fontStyle: styleModel.fontStyle,
    fontSize: `${fontSize * 100}cqh`,
    color: styleModel.primaryColor,
    textAlign: styleModel.alignment as React.CSSProperties["textAlign"],
    whiteSpace: "pre-wrap", // Respect \n in cues
    overflowWrap: "anywhere",
    lineHeight: 1.15,
  };

  // Outline / Stroke (CSS limitation: center stroke vs FFmpeg outer stroke)
  if (styleModel.outlineColor && styleModel.outlineWidthVh) {
    textStyle.WebkitTextStroke = `${styleModel.outlineWidthVh * 100}cqh ${styleModel.outlineColor}`;
  }

  // Background box
  if (styleModel.backgroundColor) {
    textStyle.backgroundColor = styleModel.backgroundColor;
    textStyle.padding = "1cqh 2cqh";
    textStyle.borderRadius = "0.5cqh";
  }

  return (
    <div style={containerStyle}>
      <span style={textStyle}>{activeCue.text}</span>
    </div>
  );
};
