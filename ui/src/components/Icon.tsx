import type { SVGProps } from "react";

/**
 * 云笈内部图标集：统一 24px 网格、圆角端点与 1.75px 线宽。
 * 使用 currentColor，因而可直接跟随浅色/深色主题和控件状态变化。
 */
export type IconName =
  | "desktop"
  | "taskbar"
  | "performance"
  | "shield"
  | "sparkles"
  | "audio"
  | "translate"
  | "settings"
  | "info"
  | "close"
  | "eye"
  | "sun"
  | "monitor"
  | "moon"
  | "cpu"
  | "gpu"
  | "memory"
  | "network"
  | "chevron-up"
  | "chevron-down"
  | "minus"
  | "plus"
  | "check"
  | "warning"
  | "error";

interface IconProps extends Omit<SVGProps<SVGSVGElement>, "children"> {
  name: IconName;
  size?: number;
}

export function Icon({ name, size = 20, ...props }: IconProps) {
  const common = {
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.75,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
  };
  const svgProps = { width: size, height: size, viewBox: "0 0 24 24", "aria-hidden": true, ...props };

  switch (name) {
    case "desktop": return <svg {...svgProps} {...common}><rect x="3" y="4" width="18" height="13" rx="2" /><path d="M8 21h8M12 17v4M7 9l2.1 2.1L13 7.2" /></svg>;
    case "taskbar": return <svg {...svgProps} {...common}><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M3 16h18M7 18h.01M11 18h.01M15 18h3" /></svg>;
    case "performance": return <svg {...svgProps} {...common}><path d="M4 18V6M4 18h16" /><path d="m7 15 3-4 3 2 4-6" /><path d="M16 7h1v1" /></svg>;
    case "shield": return <svg {...svgProps} {...common}><path d="M12 3 19 6v5c0 4.4-2.9 7.8-7 10-4.1-2.2-7-5.6-7-10V6l7-3Z" /><path d="m9.2 12 1.8 1.8 3.8-4" /></svg>;
    case "sparkles": return <svg {...svgProps} {...common}><path d="m12 3 .9 3.1L16 7l-3.1.9L12 11l-.9-3.1L8 7l3.1-.9L12 3ZM18.5 13l.5 1.5 1.5.5-1.5.5-.5 1.5-.5-1.5-1.5-.5 1.5-.5.5-1.5ZM6 14l.7 2.3L9 17l-2.3.7L6 20l-.7-2.3L3 17l2.3-.7L6 14Z" /></svg>;
    case "audio": return <svg {...svgProps} {...common}><path d="M4 14V10M8 17V7M12 20V4M16 16V8M20 14v-4" /></svg>;
    case "translate": return <svg {...svgProps} {...common}><path d="M4 5h9M8.5 3v2M6 5c.6 3.2 2.2 5.8 4.8 7.8M11 5c-.5 2-1.6 4.2-3.7 6.2M14 18h6M17 14l-3 7M17 14l3 7" /></svg>;
    case "settings": return <svg {...svgProps} {...common}><circle cx="12" cy="12" r="3" /><path d="M19 12a7.6 7.6 0 0 0-.1-1.1l2-1.5-2-3.4-2.3.9a8.1 8.1 0 0 0-1.9-1.1L14.4 3h-4l-.3 2.8a8.1 8.1 0 0 0-1.9 1.1L5.9 6l-2 3.4 2 1.5A7.6 7.6 0 0 0 5.8 12c0 .4 0 .7.1 1.1l-2 1.5 2 3.4 2.3-.9a8.1 8.1 0 0 0 1.9 1.1l.3 2.8h4l.3-2.8a8.1 8.1 0 0 0 1.9-1.1l2.3.9 2-3.4-2-1.5c.1-.4.1-.7.1-1.1Z" /></svg>;
    case "info": return <svg {...svgProps} {...common}><circle cx="12" cy="12" r="9" /><path d="M12 11v5M12 8h.01" /></svg>;
    case "close": return <svg {...svgProps} {...common}><path d="m6 6 12 12M18 6 6 18" /></svg>;
    case "eye": return <svg {...svgProps} {...common}><path d="M3 12s3.2-5 9-5 9 5 9 5-3.2 5-9 5-9-5-9-5Z" /><circle cx="12" cy="12" r="2" /></svg>;
    case "sun": return <svg {...svgProps} {...common}><circle cx="12" cy="12" r="3.5" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></svg>;
    case "monitor": return <svg {...svgProps} {...common}><rect x="3" y="4" width="18" height="12" rx="2" /><path d="M9 20h6M12 16v4" /></svg>;
    case "moon": return <svg {...svgProps} {...common}><path d="M20 15.2A8.5 8.5 0 0 1 8.8 4 8.5 8.5 0 1 0 20 15.2Z" /></svg>;
    case "cpu": return <svg {...svgProps} {...common}><rect x="7" y="7" width="10" height="10" rx="2" /><path d="M9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3" /></svg>;
    case "gpu": return <svg {...svgProps} {...common}><rect x="3" y="6" width="15" height="12" rx="2" /><circle cx="10.5" cy="12" r="3" /><path d="M18 10h3v4h-3M6 18v2M10 18v2M14 18v2" /></svg>;
    case "memory": return <svg {...svgProps} {...common}><path d="M4 7h16v10H4zM8 10h5M8 14h3M17 10v4" /><path d="M7 4v3M11 4v3M15 4v3M7 17v3M11 17v3M15 17v3" /></svg>;
    case "network": return <svg {...svgProps} {...common}><path d="M5 17V7M2 10l3-3 3 3M19 7v10M16 14l3 3 3-3" /></svg>;
    case "chevron-up": return <svg {...svgProps} {...common}><path d="m6 14 6-6 6 6" /></svg>;
    case "chevron-down": return <svg {...svgProps} {...common}><path d="m6 10 6 6 6-6" /></svg>;
    case "minus": return <svg {...svgProps} {...common}><path d="M5 12h14" /></svg>;
    case "plus": return <svg {...svgProps} {...common}><path d="M12 5v14M5 12h14" /></svg>;
    case "check": return <svg {...svgProps} {...common}><path d="m5 12 4 4L19 6" /></svg>;
    case "warning": return <svg {...svgProps} {...common}><path d="M12 3 22 20H2L12 3Z" /><path d="M12 9v4M12 17h.01" /></svg>;
    case "error": return <svg {...svgProps} {...common}><circle cx="12" cy="12" r="9" /><path d="m9 9 6 6m0-6-6 6" /></svg>;
  }
}
