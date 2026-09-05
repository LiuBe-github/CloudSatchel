import { useEffect, useMemo, useState } from "react";
import type { PerfSnapshot } from "../vite-env";
import { getPerfSnapshot } from "../lib/bridge";
import { Switch } from "./Switch";

type PerfSection = "cpu" | "gpu" | "memory" | "network";

const SECTIONS: Array<{ id: PerfSection; icon: string; label: string }> = [
  { id: "cpu", icon: "◍", label: "CPU" },
  { id: "gpu", icon: "▣", label: "GPU" },
  { id: "memory", icon: "▤", label: "内存" },
  { id: "network", icon: "⇅", label: "网络" },
];

const MAX_POINTS = 60;

interface Series {
  points: number[];
  color: string;
}

function LineChart({ series }: { series: Series[] }) {
  const all = series.flatMap((s) => s.points);
  const max = Math.max(1, ...all);

  const toPoints = (points: number[]): string => {
    const values = points.length === 0 ? [0] : points;
    if (values.length === 1) values.push(values[0]);
    return values
      .map((value, index) => {
        const x = (index / (values.length - 1)) * 100;
        const y = 40 - (value / max) * 36 - 2;
        return `${x.toFixed(2)},${Math.max(2, Math.min(38, y)).toFixed(2)}`;
      })
      .join(" ");
  };

  return (
    <svg className="perf-chart" viewBox="0 0 100 40" preserveAspectRatio="none" aria-hidden="true">
      <line x1="0" y1="38" x2="100" y2="38" className="perf-chart-grid" />
      <line x1="0" y1="20" x2="100" y2="20" className="perf-chart-grid" />
      <line x1="0" y1="2" x2="100" y2="2" className="perf-chart-grid" />
      {series.map((s, i) => (
        <polyline
          key={i}
          points={toPoints(s.points)}
          fill="none"
          vectorEffect="non-scaling-stroke"
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
          style={{ stroke: s.color }}
        />
      ))}
    </svg>
  );
}

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 100 ? 0 : 1)} ${units[unit]}`;
}

function formatRate(bytesPerSec: number): string {
  if (!Number.isFinite(bytesPerSec) || bytesPerSec <= 0) return "0 B/s";
  return `${formatBytes(bytesPerSec)}/s`;
}

function formatPercent(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return "不可用";
  return `${value.toFixed(1)}%`;
}

function formatTemperature(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return "不可用";
  return `${value.toFixed(0)}°C`;
}

function formatMhz(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return "不可用";
  return `${value.toFixed(0)} MHz`;
}

function DetailRow({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone?: "good" | "warm" | "hot";
}) {
  return (
    <div className="perf-detail-row">
      <span className="perf-detail-label">{label}</span>
      <span className={`perf-detail-value${tone ? ` tone-${tone}` : ""}`}>{value}</span>
    </div>
  );
}

function temperatureTone(value: number | null | undefined): "good" | "warm" | "hot" | undefined {
  if (value === null || value === undefined || !Number.isFinite(value)) return undefined;
  if (value < 60) return "good";
  if (value < 80) return "warm";
  return "hot";
}

interface PerformancePanelProps {
  enabled: boolean;
  busy: boolean;
  onChange: () => void;
  /** 采样/刷新间隔（毫秒） */
  intervalMs: number;
  onIntervalChange: (ms: number) => void;
}

const INTERVAL_OPTIONS: Array<{ value: number; label: string }> = [
  { value: 200, label: "200ms" },
  { value: 500, label: "500ms" },
  { value: 1000, label: "1000ms" },
];

export function PerformancePanel({
  enabled,
  busy,
  onChange,
  intervalMs,
  onIntervalChange,
}: PerformancePanelProps) {
  const [section, setSection] = useState<PerfSection>("cpu");
  const [snapshot, setSnapshot] = useState<PerfSnapshot | null>(null);
  const [history, setHistory] = useState<PerfSnapshot[]>([]);

  useEffect(() => {
    if (!enabled) {
      setSnapshot(null);
      setHistory([]);
      return;
    }
    let alive = true;
    const tick = async () => {
      try {
        const next = await getPerfSnapshot();
        if (!alive || !next) return;
        setSnapshot(next);
        setHistory((prev) => [...prev.slice(-(MAX_POINTS - 1)), next]);
      } catch {
        /* 采样失败时保留上一帧 */
      }
    };
    void tick();
    const timer = window.setInterval(tick, intervalMs);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [enabled, intervalMs]);

  const chartSeries = useMemo<Series[]>(() => {
    if (!snapshot) return [];
    switch (section) {
      case "cpu":
        return [
          {
            points: history.map((s) => s.cpu.usage),
            color: "var(--color-bamboo)",
          },
        ];
      case "gpu":
        return [
          {
            points: history.map((s) => s.gpu.utilization ?? 0),
            color: "var(--color-bamboo-light)",
          },
        ];
      case "memory":
        return [
          {
            points: history.map((s) => s.memory.usage),
            color: "var(--color-bamboo)",
          },
        ];
      case "network":
        return [
          {
            points: history.map((s) => s.network.downloadBytesPerSec),
            color: "var(--color-bamboo)",
          },
          {
            points: history.map((s) => s.network.uploadBytesPerSec),
            color: "var(--color-bamboo-light)",
          },
        ];
    }
  }, [history, section, snapshot]);

  const heroValue = useMemo(() => {
    if (!snapshot) return "—";
    switch (section) {
      case "cpu":
        return formatPercent(snapshot.cpu.usage);
      case "gpu":
        return formatPercent(snapshot.gpu.utilization);
      case "memory":
        return formatPercent(snapshot.memory.usage);
      case "network":
        return formatRate(snapshot.network.downloadBytesPerSec);
    }
  }, [section, snapshot]);

  const heroCaption = useMemo(() => {
    if (!snapshot) return "等待采样…";
    switch (section) {
      case "cpu":
        return `温度 ${formatTemperature(snapshot.cpu.temperature)}`;
      case "gpu":
        return snapshot.gpu.name ?? "未检测到可读取的 GPU";
      case "memory":
        return `${formatBytes(snapshot.memory.usedBytes)} / ${formatBytes(snapshot.memory.totalBytes)}`;
      case "network":
        return `上行 ${formatRate(snapshot.network.uploadBytesPerSec)}`;
    }
  }, [section, snapshot]);

  const renderDetails = () => {
    if (!snapshot) {
      return (
        <div className="perf-empty">
          {enabled ? "正在读取本机性能数据…" : "开启功能后开始实时采集本机性能"}
        </div>
      );
    }

    if (section === "cpu") {
      const cpu = snapshot.cpu;
      return (
        <div className="perf-details">
          <DetailRow label="CPU 占用率" value={formatPercent(cpu.usage)} />
          <DetailRow
            label="CPU 温度"
            value={formatTemperature(cpu.temperature)}
            tone={temperatureTone(cpu.temperature)}
          />
          <DetailRow label="当前频率" value={formatMhz(cpu.currentFrequencyMhz)} />
          <DetailRow label="基准频率" value={formatMhz(cpu.baseFrequencyMhz)} />
          <DetailRow label="物理核心" value={cpu.coreCount?.toString() ?? "不可用"} />
          <DetailRow label="逻辑处理器" value={cpu.logicalProcessorCount.toString()} />
          <DetailRow label="进程数" value={cpu.processCount.toString()} />
          <DetailRow label="线程数" value={cpu.threadCount.toString()} />
        </div>
      );
    }

    if (section === "gpu") {
      const gpu = snapshot.gpu;
      return (
        <div className="perf-details">
          <DetailRow label="GPU 型号" value={gpu.name ?? "不可用"} />
          <DetailRow label="GPU 占用率" value={formatPercent(gpu.utilization)} />
          <DetailRow
            label="GPU 温度"
            value={formatTemperature(gpu.temperature)}
            tone={temperatureTone(gpu.temperature)}
          />
          <DetailRow
            label="显存使用"
            value={gpu.memoryUsedMb === null ? "不可用" : `${gpu.memoryUsedMb.toFixed(0)} MB`}
          />
          <DetailRow
            label="显存总量"
            value={gpu.memoryTotalMb === null ? "不可用" : `${gpu.memoryTotalMb.toFixed(0)} MB`}
          />
          <DetailRow
            label="共享显存"
            value={
              gpu.sharedMemoryUsedMb === null
                ? "不可用"
                : `${gpu.sharedMemoryUsedMb.toFixed(0)} MB`
            }
          />
          <DetailRow label="驱动版本" value={gpu.driverVersion ?? "不可用"} />
        </div>
      );
    }

    if (section === "memory") {
      const mem = snapshot.memory;
      return (
        <div className="perf-details">
          <DetailRow label="内存占用率" value={formatPercent(mem.usage)} />
          <DetailRow label="已使用" value={formatBytes(mem.usedBytes)} />
          <DetailRow label="可用" value={formatBytes(mem.availableBytes)} />
          <DetailRow label="总容量" value={formatBytes(mem.totalBytes)} />
          <DetailRow
            label="提交内存"
            value={
              mem.pagefileUsedBytes === null
                ? "不可用"
                : formatBytes(mem.pagefileUsedBytes)
            }
          />
          <DetailRow
            label="分页池总量"
            value={
              mem.pagefileTotalBytes === null
                ? "不可用"
                : formatBytes(mem.pagefileTotalBytes)
            }
          />
        </div>
      );
    }

    const net = snapshot.network;
    return (
      <div className="perf-details">
        <DetailRow label="下行速率" value={formatRate(net.downloadBytesPerSec)} />
        <DetailRow label="上行速率" value={formatRate(net.uploadBytesPerSec)} />
        <DetailRow label="网络适配器" value={net.adapterName ?? "不可用"} />
        <DetailRow
          label="链路速率"
          value={net.linkSpeedMbps === null ? "不可用" : `${net.linkSpeedMbps} Mbps`}
        />
      </div>
    );
  };

  return (
    <div className="detail-card noise-bg">
      <div className="detail-hero">
        <div className="detail-icon">▥</div>
        <div className="detail-titles">
          <h1 className="detail-title">主机性能监控</h1>
          <p className="detail-subtitle">参考 Windows 任务管理器性能页，实时查看 CPU、GPU、内存与网络</p>
        </div>
      </div>

      <div className="detail-rule" />

      <div className="detail-row perf-switch-row">
        <div className="detail-state">
          <span className={`state-dot ${enabled ? "on" : "off"}`} />
          <div>
            <div className="state-label">{enabled ? "功能已激活" : "功能已停用"}</div>
            <div className="state-hint">
              {enabled ? `正在本地采集 · 刷新间隔 ${intervalMs}ms` : "开启后开始实时监控"}
            </div>
          </div>
        </div>
        <div className="perf-switch-group">
          <select
            className="select-box"
            value={intervalMs}
            onChange={(e) => onIntervalChange(Number(e.target.value))}
            title="刷新间隔"
          >
            {INTERVAL_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
          <Switch checked={enabled} onChange={onChange} disabled={busy} />
        </div>
      </div>

      <div className="perf-layout">
        <nav className="perf-nav">
          {SECTIONS.map((item) => (
            <button
              key={item.id}
              className={`perf-nav-item ${item.id === section ? "active" : ""}`}
              onClick={() => setSection(item.id)}
            >
              <span className="perf-nav-icon">{item.icon}</span>
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        <div className="perf-main">
          <div className="perf-hero">
            <div>
              <div className="perf-hero-label">
                {section === "cpu"
                  ? "CPU 总占用率"
                  : section === "gpu"
                    ? "GPU 利用率"
                    : section === "memory"
                      ? "内存占用率"
                      : "下行速率"}
              </div>
              <div className="perf-hero-value">{heroValue}</div>
              <div className="perf-hero-caption">{heroCaption}</div>
            </div>
          </div>

          <div className="perf-chart-wrap">
            {enabled ? <LineChart series={chartSeries} /> : <div className="perf-empty">采样已停止</div>}
          </div>
          {section === "network" && enabled && snapshot && (
            <div className="perf-legend">
              <span><i style={{ background: "var(--color-bamboo)" }} /> 下行</span>
              <span><i style={{ background: "var(--color-bamboo-light)" }} /> 上行</span>
            </div>
          )}
          <div className="perf-detail-list">{renderDetails()}</div>
        </div>
      </div>
    </div>
  );
}
