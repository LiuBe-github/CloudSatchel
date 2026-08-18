import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow, PhysicalPosition, primaryMonitor } from "@tauri-apps/api/window";
import {
  getState,
  inTauri,
  setPetEnabled,
  setPetPosition,
} from "../lib/bridge";

/**
 * 虚拟桌宠（FR-16）：桌面常驻 2D 精灵。
 * - 精灵为应用内 CSS 自绘（竹灵小猫，无第三方素材、无版权风险、随包内嵌不联网）
 * - 左键拖拽移动，位置持久化（重启保持）
 * - 双击 / 右键弹出小菜单：隐藏桌宠（临时隐藏，开关保持）、退出桌宠（关闭功能）
 * - 置顶显示、跳过任务栏；隐私操作（FR-13）触发时由后端联动隐藏，恢复后还原
 */
export default function PetWindow() {
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [ready, setReady] = useState(false);
  const lastClick = useRef(0);

  // 初始化：应用持久化位置或底部默认位置；开关开启才显示
  useEffect(() => {
    if (!inTauri()) return;
    const win = getCurrentWindow();
    void getState().then(async (s) => {
      if (!s.petEnabled) return;
      if (s.petX >= 0 && s.petY >= 0) {
        await win.setPosition(new PhysicalPosition(s.petX, s.petY));
      } else {
        // 默认：主屏工作区底部中间偏右（贴底）
        try {
          const primary = await primaryMonitor();
          if (primary) {
            const size = await win.outerSize();
            const wa = primary.workArea;
            const x = wa.position.x + wa.size.width - size.width - 80;
            const y = wa.position.y + wa.size.height - size.height - 6;
            await win.setPosition(new PhysicalPosition(Math.max(0, x), Math.max(0, y)));
          }
        } catch {
          /* 忽略定位失败 */
        }
      }
      await win.show();
      setReady(true);
    });
  }, []);

  // 拖拽结束后持久化位置
  useEffect(() => {
    if (!inTauri()) return;
    const win = getCurrentWindow();
    const unlisten = win.onMoved(({ payload }) => {
      void setPetPosition(payload.x, payload.y);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const hide = useCallback(() => {
    setMenu(null);
    if (inTauri()) void getCurrentWindow().hide();
  }, []);

  const quit = useCallback(async () => {
    setMenu(null);
    if (inTauri()) {
      await setPetEnabled(false);
      await getCurrentWindow().hide();
    }
  }, []);

  // 双击检测（两次点击间隔 < 350ms）→ 弹出菜单
  const onPointerDown = useCallback(() => {
    const now = Date.now();
    if (now - lastClick.current < 350) {
      setMenu({ x: 60, y: 40 });
    }
    lastClick.current = now;
  }, []);

  const onContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setMenu({ x: e.clientX, y: e.clientY });
  }, []);

  return (
    <div
      className={`pet-shell${ready ? " ready" : ""}`}
      onPointerDown={onPointerDown}
      onContextMenu={onContextMenu}
      data-tauri-drag-region
    >
      <div className="pet-sprite">
        <div className="pet-ear pet-ear-l" />
        <div className="pet-ear pet-ear-r" />
        <div className="pet-head">
          <div className="pet-eye pet-eye-l" />
          <div className="pet-eye pet-eye-r" />
          <div className="pet-muzzle" />
          <div className="pet-blush pet-blush-l" />
          <div className="pet-blush pet-blush-r" />
        </div>
        <div className="pet-body">
          <div className="pet-paw pet-paw-l" />
          <div className="pet-paw pet-paw-r" />
        </div>
        <div className="pet-tail" />
      </div>
      <div className="pet-shadow" />

      {menu && (
        <>
          <div className="pet-menu-mask" onClick={() => setMenu(null)} />
          <div className="pet-menu" style={{ left: menu.x, top: menu.y }}>
            <button className="pet-menu-item" onClick={hide}>
              <span className="pet-menu-icon">🙈</span> 隐藏桌宠
            </button>
            <button className="pet-menu-item" onClick={() => void quit()}>
              <span className="pet-menu-icon">👋</span> 退出桌宠
            </button>
          </div>
        </>
      )}
    </div>
  );
}
