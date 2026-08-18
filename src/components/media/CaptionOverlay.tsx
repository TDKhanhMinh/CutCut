import React, { useEffect, useState, useMemo } from 'react';
import { CaptionCue, CaptionStyle, EditPlan } from '@/types/project';

interface CaptionOverlayProps {
    videoRef: React.RefObject<HTMLVideoElement | null>;
    cues: CaptionCue[];
    styleModel: CaptionStyle | null;
    editPlan?: EditPlan | null;
}

export const CaptionOverlay: React.FC<CaptionOverlayProps> = ({
    videoRef,
    cues,
    styleModel,
    editPlan,
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
    const activeCue = useMemo(() => {
        return cues.find((c) => currentTimeMs >= c.startMs && currentTimeMs <= c.endMs);
    }, [cues, currentTimeMs]);

    // Check if we are inside an enabled cut
    const isInsideCut = useMemo(() => {
        if (!editPlan) return false;
        return editPlan.actions.some((a) => {
            if (!a.enabled || a.payload.type !== 'cut') return false;
            return currentTimeMs >= a.payload.startMs && currentTimeMs < a.payload.endMs;
        });
    }, [editPlan, currentTimeMs]);

    if (!styleModel || !activeCue || isInsideCut) {
        return null;
    }

    // Convert CaptionStyle to React CSS
    // Container query (`@container`) is required on the parent wrapper for `cqh` to work.
    const containerStyle: React.CSSProperties = {
        position: 'absolute',
        left: `${styleModel.positionXVw * 100}%`,
        top: `${styleModel.positionYVh * 100}%`,
        transform: 'translate(-50%, -100%)', // Align bottom-center
        width: '100%',
        pointerEvents: 'none',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 10,
    };

    // Override transform based on alignment
    if (styleModel.alignment === 'left') {
        containerStyle.transform = 'translate(0%, -100%)';
        containerStyle.alignItems = 'flex-start';
    } else if (styleModel.alignment === 'right') {
        containerStyle.transform = 'translate(-100%, -100%)';
        containerStyle.alignItems = 'flex-end';
    }

    // Map properties
    const textStyle: React.CSSProperties = {
        fontFamily: styleModel.fontFamily,
        fontWeight: styleModel.fontWeight,
        fontStyle: styleModel.fontStyle,
        fontSize: `${styleModel.fontSizeVh * 100}cqh`,
        color: styleModel.primaryColor,
        textAlign: styleModel.alignment as any,
        whiteSpace: 'pre-wrap', // Respect \n in cues
    };

    // Outline / Stroke (CSS limitation: center stroke vs FFmpeg outer stroke)
    if (styleModel.outlineColor && styleModel.outlineWidthVh) {
        textStyle.WebkitTextStroke = `${styleModel.outlineWidthVh * 100}cqh ${styleModel.outlineColor}`;
    }

    // Background box
    if (styleModel.backgroundColor) {
        textStyle.backgroundColor = styleModel.backgroundColor;
        textStyle.padding = '1cqh 2cqh';
        textStyle.borderRadius = '0.5cqh';
    }

    return (
        <div style={containerStyle}>
            <span style={textStyle}>
                {activeCue.text}
            </span>
        </div>
    );
};
