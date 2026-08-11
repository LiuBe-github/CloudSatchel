interface SwitchProps {
  checked: boolean;
  onChange: () => void;
  disabled?: boolean;
}

export function Switch({ checked, onChange, disabled }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      className={`switch ${checked ? "on" : "off"}`}
      onClick={onChange}
      disabled={disabled}
      aria-label="功能开关"
    >
      <span className="switch-thumb" />
    </button>
  );
}
