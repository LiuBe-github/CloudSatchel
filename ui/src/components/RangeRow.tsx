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
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </div>
  );
}
