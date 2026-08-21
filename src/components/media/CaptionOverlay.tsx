import React, { useEffect, useState, useMemo } from "react";
import { CaptionCue, CaptionStyle, EditPlan } from "@/types/project";
import { buildCutIndex, findActiveCut } from "@/hooks/useCutPreview";
import { CAPTION_SAFE_AREA, mapCaptionStyleToOverlay } from "@/lib/caption-style";

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

  const overlayStyle = useMemo(
    () => (styleModel ? mapCaptionStyleToOverlay(styleModel) : null),
    [styleModel],
  );

  const cutIndex = useMemo(
    () => (editPlan ? buildCutIndex(editPlan, sourceMediaId) : []),
    [editPlan, sourceMediaId],
  );

  // Check if we are inside an enabled cut
  const isInsideCut = useMemo(() => {
    return Boolean(findActiveCut(cutIndex, currentTimeMs));
  }, [cutIndex, currentTimeMs]);

  if (!overlayStyle || !activeCue || isInsideCut) {
    return null;
  }

  // Convert CaptionStyle to React CSS with the same normalized safe-area
  // semantics used by the native mapper. The overlay width is bounded so
  // left/right alignment cannot silently overflow the preview frame.
  // Container query (`@container`) is required on the parent wrapper for `cqh` to work.
  const containerStyle: React.CSSProperties = {
    position: "absolute",
    left: `${overlayStyle.positionX * 100}%`,
    top: `${overlayStyle.positionY * 100}%`,
    transform: "translate(-50%, -100%)", // Align bottom-center
    width: `${CAPTION_SAFE_AREA.maxWidth * 100}%`,
    maxWidth: `${CAPTION_SAFE_AREA.maxWidth * 100}%`,
    maxHeight: "80%",
    overflow: "hidden",
    pointerEvents: "none",
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    justifyContent: "center",
    zIndex: 10,
  };

  // Override transform based on alignment
  if (overlayStyle.alignment === "left") {
    containerStyle.transform = "translate(0%, -100%)";
    containerStyle.alignItems = "flex-start";
  } else if (overlayStyle.alignment === "right") {
    containerStyle.transform = "translate(-100%, -100%)";
    containerStyle.alignItems = "flex-end";
  }

  // Map properties
  const textStyle: React.CSSProperties = {
    fontFamily: overlayStyle.fontFamily,
    fontWeight: overlayStyle.fontWeight,
    fontStyle: overlayStyle.fontStyle,
    fontSize: `${overlayStyle.fontSize * 100}cqh`,
    color: overlayStyle.primaryColor,
    textAlign: overlayStyle.alignment,
    whiteSpace: "pre-wrap", // Respect \n in cues
    overflowWrap: "anywhere",
    lineHeight: 1.15,
  };

  // Outline / Stroke (CSS limitation: center stroke vs FFmpeg outer stroke)
  if (overlayStyle.outlineColor && overlayStyle.outlineWidth > 0) {
    textStyle.WebkitTextStroke = `${overlayStyle.outlineWidth * 100}cqh ${overlayStyle.outlineColor}`;
  }

  // Background box
  if (overlayStyle.backgroundColor) {
    textStyle.backgroundColor = overlayStyle.backgroundColor;
    textStyle.padding = "1cqh 2cqh";
    textStyle.borderRadius = "0.5cqh";
  }

  return (
    <div style={containerStyle}>
      <span style={textStyle} aria-live="polite">
        {activeCue.text}
      </span>
    </div>
  );
};
