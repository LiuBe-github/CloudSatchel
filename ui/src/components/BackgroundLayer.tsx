import { useEffect, useState } from "react";
import { readBackgroundImage } from "../lib/bridge";
import type { AppState } from "../vite-env";

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

interface BackgroundLayerProps {
  state: AppState;
}

export function BackgroundLayer({ state }: BackgroundLayerProps) {
  const path = state.backgroundImagePath?.trim() ?? "";
  const [src, setSrc] = useState("");

  useEffect(() => {
    let cancelled = false;
    if (!path) {
      setSrc("");
      return;
    }
    readBackgroundImage(path)
      .then((dataUrl) => {
        if (!cancelled) setSrc(dataUrl ?? "");
      })
      .catch(() => {
        if (!cancelled) setSrc("");
      });
    return () => {
      cancelled = true;
    };
  }, [path]);

  if (!path || !src) return null;

  const fit = state.backgroundFit ?? "cover";
  const dim = clamp(state.backgroundDim ?? 0.25, 0, 1);
  const blur = clamp(state.backgroundBlur ?? 0, 0, 20);
  const scale = clamp(state.backgroundScale ?? 1, 0.5, 2);
  const positionX = clamp(state.backgroundPositionX ?? 50, 0, 100);
  const positionY = clamp(state.backgroundPositionY ?? 50, 0, 100);

  const imageStyle: React.CSSProperties = {
    objectPosition: `${positionX}% ${positionY}%`,
    filter: blur > 0 ? `blur(${blur}px)` : undefined,
    transform: `scale(${scale})`,
    transformOrigin: `${positionX}% ${positionY}%`,
  };

  return (
    <div className="background-layer">
      {fit === "repeat" ? (
        <div
          className="background-repeat"
          style={{
            backgroundImage: `url("${src}")`,
            backgroundPosition: `${positionX}% ${positionY}%`,
            ...imageStyle,
          }}
        />
      ) : (
        <img
          className="background-img"
          src={src}
          alt=""
          style={{ objectFit: fit === "contain" ? "contain" : "cover", ...imageStyle }}
        />
      )}
      <div className="background-dim" style={{ opacity: dim }} />
    </div>
  );
}
