interface RangeRowProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (value: number) => string;
  onChange: (value: number) => void;
}

export function RangeRow({ label, value, min, max, step, format, onChange }: RangeRowProps) {
  // 已走部分用竹青渐变填充（HIG 滑杆：进度可视化）
  const pct = max > min ? ((value - min) / (max - min)) * 100 : 0;
  const trackStyle: React.CSSProperties = {
    background: `linear-gradient(90deg, var(--color-bamboo) ${pct}%, var(--color-paper-deep) ${pct}%)`,
  };
  return (
    <div className="range-row">
      <div className="range-head">
        <span className="range-label">{label}</span>
        <span className="range-value">{format(value)}</span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        style={trackStyle}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </div>
  );
}
