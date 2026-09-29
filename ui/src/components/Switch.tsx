interface SwitchProps {
  checked: boolean;
  onChange: () => void;
  disabled?: boolean;
  /** 供读屏和悬停提示使用的具体控制名称。 */
  label: string;
  /** 异步请求进行中时显示小型忙碌指示，但不改变控件尺寸。 */
  busy?: boolean;
}

export function Switch({ checked, onChange, disabled, label, busy = false }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      className={`switch ${checked ? "on" : "off"}`}
      onClick={onChange}
      disabled={disabled}
      aria-label={label}
      aria-busy={busy || undefined}
      title={`${label}：${checked ? "已开启" : "已关闭"}`}
    >
      <span className="switch-thumb" />
      {busy && <span className="switch-spinner" aria-hidden="true" />}
    </button>
  );
}
